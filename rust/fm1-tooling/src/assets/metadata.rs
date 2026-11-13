// SPDX-License-Identifier: GPL-3.0-only
//! Parameter-source grammar and editor metadata assembly. Display labels still
//! arrive from the explicitly transitional C exporter; numeric specs and FM6
//! routing use the same Rust tables as the instrument.
use super::{assertion, manual::error, site};
use crate::{text, Error, Result};
use fancy_regex::Regex;
use serde_json::{json, Map, Value};

const LANES: [&str; 6] = ["Kick", "Snare", "Clap", "Hat", "Open hat", "Perc"];
const LAYERS: [&str; 5] = ["Pad", "Bass", "Acid", "Pluck", "Drums"];
fn get<'a>(v: &'a Value, key: &str) -> Result<&'a Value> {
    v.get(key).ok_or_else(|| error("KeyError", key))
}
fn integer(v: &Value) -> Result<i64> {
    v.as_i64()
        .ok_or_else(|| Error::value("expected metadata integer"))
}
fn re(p: &str) -> Result<Regex> {
    Regex::new(p).map_err(|e| Error::value(e.to_string()))
}
pub fn strip_comments(s: &str) -> Result<String> {
    site::sub(r"(?s)/\*.*?\*/", s, "").and_then(|s| site::sub(r"//[^\n]*", &s, ""))
}
struct Expr<'a> {
    input: &'a [u8],
    at: usize,
    names: &'a Map<String, Value>,
    depth: usize,
}
impl Expr<'_> {
    fn skip(&mut self) {
        while self.input.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn expr(&mut self, mul: bool) -> Result<i64> {
        let mut a = if mul { self.atom()? } else { self.expr(true)? };
        loop {
            self.skip();
            let op = self.input.get(self.at).copied().unwrap_or(0);
            if (mul && op != b'*') || (!mul && op != b'+' && op != b'-') {
                break;
            }
            self.at += 1;
            let b = if mul { self.atom()? } else { self.expr(true)? };
            a = match op {
                b'*' => a.checked_mul(b),
                b'+' => a.checked_add(b),
                _ => a.checked_sub(b),
            }
            .ok_or_else(|| Error::value("parameter expression overflow"))?;
        }
        Ok(a)
    }
    fn atom(&mut self) -> Result<i64> {
        self.skip();
        self.depth += 1;
        if self.depth > 64 {
            return Err(Error::value("parameter expression depth exceeds64"));
        }
        let c = self.input.get(self.at).copied().unwrap_or(0);
        let value = match c {
            b'+' | b'-' => {
                self.at += 1;
                let n = self.atom()?;
                if c == b'-' {
                    n.checked_neg()
                        .ok_or_else(|| Error::value("parameter expression overflow"))?
                } else {
                    n
                }
            }
            b'(' => {
                self.at += 1;
                let n = self.expr(false)?;
                self.skip();
                if self.input.get(self.at) != Some(&b')') {
                    return Err(error("SyntaxError", "invalid parameter expression"));
                }
                self.at += 1;
                n
            }
            _ => {
                let start = self.at;
                while self
                    .input
                    .get(self.at)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    self.at += 1;
                }
                let token = std::str::from_utf8(&self.input[start..self.at])
                    .map_err(|_| Error::value("non-ASCII parameter expression"))?;
                if c.is_ascii_digit() {
                    if let Some(n) = token
                        .strip_prefix("0x")
                        .or_else(|| token.strip_prefix("0X"))
                    {
                        i64::from_str_radix(n, 16)
                    } else {
                        token.parse()
                    }
                    .map_err(|_| error("SyntaxError", "invalid parameter expression"))?
                } else if token == "EN_NLANE" {
                    6
                } else {
                    integer(self.names.get(token).ok_or_else(|| {
                        error("NameError", format!("name '{token}' is not defined"))
                    })?)?
                }
            }
        };
        self.depth -= 1;
        Ok(value)
    }
}
pub fn param_enum(s: &str) -> Result<Value> {
    site::bounded(s)?;
    let end = s
        .find("P_NPARAMS")
        .ok_or_else(|| Error::value("substring not found"))?;
    let start = s[..end]
        .rfind("enum {")
        .ok_or_else(|| Error::value("substring not found"))?
        + 6;
    let body = strip_comments(&s[start..end + 9])?;
    let mut names = Map::new();
    let mut cur = 0_i64;
    for tok in body
        .split(',')
        .map(super::manual::trim)
        .filter(|s| !s.is_empty())
    {
        if names.len() >= 1024 {
            return Err(Error::value("parameter enum exceeds1024 entries"));
        }
        let name = if let Some((name, expr)) = tok.split_once('=') {
            let mut p = Expr {
                input: expr.as_bytes(),
                at: 0,
                names: &names,
                depth: 0,
            };
            cur = p.expr(false)?;
            p.skip();
            if p.at != p.input.len() {
                return Err(error("SyntaxError", "unsupported parameter expression"));
            }
            super::manual::trim(name)
        } else {
            tok
        };
        names.insert(name.into(), json!(cur));
        cur = cur
            .checked_add(1)
            .ok_or_else(|| Error::value("parameter enum overflow"))?;
    }
    Ok(Value::Object(names))
}
fn block<'a>(s: &'a str, name: &str) -> Result<&'a str> {
    let p = format!(r"{}\[[^\]]*\](?:\[[^\]]*\])?\s*=\s*\{{", name);
    let m = re(&p)?
        .find(s)
        .map_err(|e| Error::value(e.to_string()))?
        .ok_or_else(|| error("AttributeError", "'NoneType' object has no attribute 'end'"))?;
    let i = m.end();
    let mut depth = 1;
    // The original parser advances before inspecting the next character.
    for (j, c) in s[i..].char_indices().skip(1) {
        if c == '{' {
            depth += 1;
        }
        if c == '}' {
            depth -= 1;
        }
        if depth == 0 {
            return Ok(&s[i..i + j]);
        }
    }
    Err(error("IndexError", "string index out of range"))
}
fn captures(s: &str, p: &str) -> Result<Vec<String>> {
    re(p)?
        .captures_iter(s)
        .map(|v| {
            let c = v.map_err(|e| Error::value(e.to_string()))?;
            Ok(c.get(1).unwrap().as_str().to_string())
        })
        .collect()
}
pub fn ui_tables(source: &str, p: &Value) -> Result<Value> {
    site::bounded(source)?;
    let t = strip_comments(source)?;
    let names = captures(block(&t, "VIEW_NAME")?, r#""([^"]*)""#)?;
    let rows = captures(block(&t, "VIEW_KNOB")?, r"\{([^{}]*)\}")?;
    let labels = captures(block(&t, "KNOB_LABEL")?, r"\{([^{}]*)\}")?;
    let mut groups = Vec::new();
    for ((n, row), lab) in names.iter().zip(rows).zip(labels) {
        let labels = captures(&lab, r#""([^"]*)""#)?;
        let mut knobs = Vec::new();
        for (k, tok) in row.split(',').map(super::manual::trim).enumerate() {
            if let Some(id) = p.get(tok) {
                knobs.push(json!({"p":id,"label":labels.get(k).map_or(tok, String::as_str)}));
            }
        }
        if !knobs.is_empty() && !["Jam", "Perform", "Visual", "Sequencer"].contains(&n.as_str()) {
            groups.push(json!({"name":n,"knobs":knobs}));
        }
    }
    Ok(json!(groups))
}
pub fn algorithms(source: &str) -> Result<Value> {
    site::bounded(source)?;
    let r = re(r"(?s)FM6_ALG\[32\]\[6\]\s*=\s*\{(.*?)\};")?;
    let c = r
        .captures(source)
        .map_err(|e| Error::value(e.to_string()))?
        .ok_or_else(|| {
            error(
                "AttributeError",
                "'NoneType' object has no attribute 'group'",
            )
        })?;
    let rows = captures(c.get(1).unwrap().as_str(), r"\{([^{}]*)\}")?;
    let values: Result<Vec<Vec<i64>>> = rows
        .iter()
        .map(|r| {
            r.split(',')
                .map(|s| {
                    i64::from_str_radix(super::manual::trim(s).trim_start_matches("0x"), 16)
                        .map_err(|_| Error::value("invalid hexadecimal algorithm value"))
                })
                .collect()
        })
        .collect();
    Ok(json!(values?))
}
fn eval(
    node: &Value,
    env: &mut Map<String, Value>,
    budget: &mut usize,
    depth: usize,
) -> Result<Value> {
    if *budget == 0 || depth > 96 {
        return Err(Error::value("parameter recipe exceeds bounded evaluation"));
    }
    *budget -= 1;
    let op = text(&node[0])?;
    let e = |v: &Value, env: &mut Map<String, Value>, budget: &mut usize| {
        eval(v, env, budget, depth + 1)
    };
    Ok(match op {
        "lit" => node[1].clone(),
        "var" => get(&Value::Object(env.clone()), text(&node[1])?)?.clone(),
        "list" => Value::Array(
            node.as_array().unwrap()[1..]
                .iter()
                .map(|v| e(v, env, budget))
                .collect::<Result<_>>()?,
        ),
        "dict" => {
            let mut m = Map::new();
            for pair in &node.as_array().unwrap()[1..] {
                let k = e(&pair[0], env, budget)?;
                m.insert(text(&k)?.into(), e(&pair[1], env, budget)?);
            }
            Value::Object(m)
        }
        "at" => {
            let a = e(&node[1], env, budget)?;
            let k = e(&node[2], env, budget)?;
            if let Some(k) = k.as_str() {
                get(&a, k)?.clone()
            } else {
                a.get(
                    usize::try_from(integer(&k)?)
                        .map_err(|_| Error::value("negative recipe index"))?,
                )
                .cloned()
                .ok_or_else(|| error("IndexError", "list index out of range"))?
            }
        }
        "add" | "mul" | "ne" => {
            let a = e(&node[1], env, budget)?;
            let b = e(&node[2], env, budget)?;
            if op == "ne" {
                json!(a != b)
            } else if op == "add" && a.is_array() && b.is_array() {
                let mut a = a.as_array().unwrap().clone();
                a.extend(b.as_array().unwrap().clone());
                json!(a)
            } else if op == "add" && a.is_string() && b.is_string() {
                json!(text(&a)?.to_owned() + text(&b)?)
            } else {
                json!(if op == "add" {
                    integer(&a)?.checked_add(integer(&b)?)
                } else {
                    integer(&a)?.checked_mul(integer(&b)?)
                }
                .ok_or_else(|| Error::value("recipe integer overflow"))?)
            }
        }
        "each" => {
            let name = text(&node[1])?;
            let rows = e(&node[2], env, budget)?;
            let previous = env.get(name).cloned();
            let mut out = Vec::new();
            for value in rows
                .as_array()
                .ok_or_else(|| Error::value("recipe iteration requires array"))?
            {
                env.insert(name.into(), value.clone());
                let mut include = true;
                for cond in &node.as_array().unwrap()[4..] {
                    include &= e(cond, env, budget)?.as_bool().unwrap_or(false);
                }
                if include {
                    out.push(e(&node[3], env, budget)?);
                }
            }
            if let Some(v) = previous {
                env.insert(name.into(), v);
            } else {
                env.remove(name);
            }
            json!(out)
        }
        "call" => {
            let mut a = Vec::new();
            for n in &node.as_array().unwrap()[2..] {
                a.push(e(n, env, budget)?);
            }
            match text(&node[1])? {
                "range" => {
                    let n = integer(&a[0])?;
                    if !(0..=1024).contains(&n) {
                        return Err(Error::value("recipe range exceeds1024"));
                    }
                    json!((0..n).collect::<Vec<_>>())
                }
                "pl" => {
                    let mut v = Vec::new();
                    for pair in a {
                        v.push(json!({"p":get(get(&Value::Object(env.clone()),"P")?,text(&pair[0])?)?,"label":pair[1]}));
                    }
                    json!(v)
                }
                "ls" => {
                    json!({"p":integer(get(env.get("P").unwrap(),text(&a[0])?)?)?+integer(&a[1])?,"label":a[2]})
                }
                _ => return Err(Error::value("unknown recipe function")),
            }
        }
        _ => return Err(Error::value("unknown parameter recipe operation")),
    })
}
pub fn assemble(
    params: &Value,
    p: &Value,
    groups: &Value,
    alg: &Value,
    names: &Value,
    product: &Value,
    canonical: bool,
) -> Result<Value> {
    let rows = params
        .as_array()
        .ok_or_else(|| Error::value("expected parameter array"))?;
    if rows.len() > 1024
        || p.as_object().is_some_and(|p| p.len() > 1024)
        || groups.as_array().is_some_and(|g| g.len() > 256)
    {
        return Err(Error::value("metadata exceeds1024 parameters/256 groups"));
    }
    if rows.len() as i64 != integer(get(p, "P_NPARAMS")?)? {
        return Err(assertion("the parameter table and the enum disagree"));
    }
    if canonical {
        if rows.len() != fm1_types::PARAMETER_COUNT
            || alg != &json!(fm1_engine::fm6::tables::FM6_ALG)
        {
            return Err(Error::value(
                "C metadata disagrees with Rust instrument tables",
            ));
        }
        for (i, (row, spec)) in rows.iter().zip(fm1_types::PARAMETERS).enumerate() {
            if row["i"] != json!(i)
                || row["n"] != json!(spec.name)
                || row["min"] != json!(spec.min)
                || row["max"] != json!(spec.max)
                || row["def"] != json!(spec.default)
            {
                return Err(Error::value(format!(
                    "C parameter {i} disagrees with Rust instrument tables"
                )));
            }
        }
    }
    let mut byname = Map::new();
    for g in groups
        .as_array()
        .ok_or_else(|| Error::value("expected UI groups"))?
    {
        byname.insert(text(&g["name"])?.into(), g["knobs"].clone());
    }
    let mut env = Map::new();
    env.insert("P".into(), p.clone());
    env.insert("byname".into(), Value::Object(byname));
    env.insert("LANES".into(), json!(LANES));
    env.insert("LAYERS".into(), json!(LAYERS));
    let recipe: Value =
        serde_json::from_str(include_str!("../../../../tools/data/parameter-tabs.json"))?;
    let mut tabs = eval(&recipe, &mut env, &mut 20000, 0)?;
    let mut placed = std::collections::BTreeSet::new();
    for tab in tabs.as_array().unwrap() {
        for section in tab["sections"].as_array().unwrap() {
            for k in section["knobs"].as_array().unwrap() {
                placed.insert(integer(&k["p"])?);
            }
        }
    }
    let left: Vec<_> = (0..rows.len())
        .filter(|i| !placed.contains(&(*i as i64)))
        .collect();
    if !left.is_empty() {
        tabs.as_array_mut().unwrap().last_mut().unwrap()["sections"].as_array_mut().unwrap().push(json!({"name":"More","knobs":left.iter().map(|i|json!({"p":i,"label":rows[*i]["n"]})).collect::<Vec<_>>()}));
    }
    let mut en = Map::new();
    for (k, v) in p
        .as_object()
        .ok_or_else(|| Error::value("expected enum object"))?
    {
        if [
            "P_CM_SND",
            "P_CM_LVL",
            "P_NPARAMS",
            "P_TEMPO",
            "P_ROOT",
            "P_MODE",
            "P_PADTONE",
            "P_AC_WAVE",
            "P_PATTERN",
            "P_TUNE",
            "P_MIDI",
            "P_THEME",
        ]
        .contains(&k.as_str())
        {
            en.insert(k.clone(), v.clone());
        }
    }
    let canonical_params = if canonical {
        let mut result = Vec::with_capacity(fm1_types::PARAMETER_COUNT);
        for (i, spec) in fm1_types::PARAMETERS.iter().enumerate() {
            let mut row = site::object(&[
                ("i", json!(i)),
                ("n", json!(spec.name)),
                ("min", json!(spec.min)),
                ("max", json!(spec.max)),
                ("def", json!(spec.default)),
            ]);
            if let Some(labels) = rows[i].get("t") {
                row.as_object_mut()
                    .unwrap()
                    .insert("t".into(), labels.clone());
            }
            result.push(row);
        }
        json!(result)
    } else {
        params.clone()
    };
    let mut out = Map::new();
    for (k, v) in [
        ("params", canonical_params),
        ("tabs", tabs.clone()),
        (
            "alg",
            if canonical {
                json!(fm1_engine::fm6::tables::FM6_ALG)
            } else {
                alg.clone()
            },
        ),
        ("lanes", json!(LANES)),
        ("layers", json!(LAYERS)),
        ("enum", Value::Object(en)),
        ("names", names.clone()),
    ] {
        out.insert(k.into(), v);
    }
    let mut data = Map::new();
    data.insert("product".into(), product.clone());
    // Preserve Python dict insertion order, including the nested fixed keys.
    data.insert(
        "parameters".into(),
        site::object(&[
            ("count", json!(rows.len())),
            (
                "userVoiceBase",
                json!(integer(get(names, "nplain")?)? + integer(get(names, "nfmfact")?)?),
            ),
            ("userSlots", get(names, "nuser")?.clone()),
        ]),
    );
    for k in ["genres", "objects", "backup"] {
        data.insert(k.into(), get(names, k)?.clone());
    }
    let generated=format!("// SPDX-License-Identifier: GPL-3.0-only\n// Generated by tools/make_params.py from the instrument tables; do not edit.\nexport const DEVICE_DATA = {};\n",site::dumps(&Value::Object(data),false,Some(2),false,false)?);
    let sections: usize = tabs
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["sections"].as_array().unwrap().len())
        .sum();
    let encoded = site::dumps(&Value::Object(out), true, None, true, false)?;
    Ok(
        json!({"params":encoded,"device":generated,"summary":[rows.len(),tabs.as_array().unwrap().len(),sections,left.len(),alg.as_array().map_or(0,Vec::len)]}),
    )
}
pub fn request(method: &str, v: &Value) -> Result<Value> {
    let a = &v["args"];
    Ok(match method {
        "strip-comments" => json!(strip_comments(text(&a[0])?)?),
        "enum" => param_enum(text(&a[0])?)?,
        "ui" => ui_tables(text(&a[0])?, &a[1])?,
        "algorithms" => algorithms(text(&a[0])?)?,
        "assemble" => assemble(
            &a[0],
            &a[1],
            &a[2],
            &a[3],
            &a[4],
            &a[5],
            a[6].as_bool().unwrap_or(false),
        )?,
        _ => return Err(Error::value("unknown metadata operation")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enum_is_checked_arithmetic_without_eval() {
        let p = param_enum("enum { A=-2, B=(A + 3)*EN_NLANE, C, P_NPARAMS };").unwrap();
        assert_eq!(p, json!({"A":-2,"B":6,"C":7,"P_NPARAMS":8}));
        assert!(param_enum("enum { A=__import__('os'), P_NPARAMS };").is_err());
        assert!(param_enum("enum { A=9223372036854775807, P_NPARAMS };").is_err());
        assert!(param_enum(&format!(
            "enum {{ A={}1{}, P_NPARAMS }};",
            "(".repeat(65),
            ")".repeat(65)
        ))
        .is_err());
    }
    #[test]
    fn ui_zip_unknown_ids_and_short_labels_match_source() {
        let ui="VIEW_NAME[2]={\n\"Tempo\",\"Jam\"}; VIEW_KNOB[2][3]={\n{P_A,unknown,P_B},{P_A}}; KNOB_LABEL[2][3]={\n{\"One\"},{\"Hidden\"}};";
        assert_eq!(
            ui_tables(ui, &json!({"P_A":0,"P_B":2})).unwrap(),
            json!([{"name":"Tempo","knobs":[{"p":0,"label":"One"},{"p":2,"label":"P_B"}]}])
        );
    }
}
