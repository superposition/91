/* SPDX-License-Identifier: GPL-3.0-only
 * Compatibility launcher. Numeric generation lives in fm1-tooling and reuses
 * fm1-engine's sine function. From the repository root:
 * gcc tools/gen_sint.c -o /tmp/gs && /tmp/gs > firmware/src/dsp/sint_tab.h
 * For a different working directory set FM1_GEN_SINT_SCRIPT to the absolute
 * tools/gen_sint.py path. FM1_PYTHON selects the Python interpreter.
 */
#include <stdlib.h>
#include <stdio.h>
#ifdef _WIN32
#include <process.h>
#else
#include <unistd.h>
#endif
int main(void)
{
    const char *script = getenv("FM1_GEN_SINT_SCRIPT");
    const char *python = getenv("FM1_PYTHON");
    if (!script) script = "tools/gen_sint.py";
#ifdef _WIN32
    if (!python) python = "python";
    return (int)_spawnlp(_P_WAIT, python, python, script, (char *)0);
#else
    if (!python) python = "python3";
    execlp(python, python, script, (char *)0);
    perror(python);
    return 1;
#endif
}
