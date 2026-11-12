// SPDX-License-Identifier: GPL-3.0-only
// Ported from the frozen C parameter table; numeric IDs are a compatibility contract.
use crate::ValueError;
pub const PARAMETER_COUNT: usize = 176;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ParamId(u8);
impl ParamId {
    pub const fn new(index: usize) -> Result<Self, ValueError> {
        if index < PARAMETER_COUNT {
            Ok(Self(index as u8))
        } else {
            Err(ValueError::Index)
        }
    }
    pub const fn index(self) -> usize {
        self.0 as usize
    }
    pub fn info(self) -> &'static ParamInfo {
        &PARAMETERS[self.index()]
    }
    pub const fn is_global(self) -> bool {
        self.0 == 73 || self.0 == 74 || self.0 == 75
    }
    pub const PAD: Self = Self(0);
    pub const ACID: Self = Self(1);
    pub const BASS: Self = Self(2);
    pub const PULSE: Self = Self(3);
    pub const NOISE: Self = Self(4);
    pub const TEMPO: Self = Self(5);
    pub const EVOLVE: Self = Self(6);
    pub const MOTION: Self = Self(7);
    pub const DRIFT: Self = Self(8);
    pub const BRIGHT: Self = Self(9);
    pub const PADTONE: Self = Self(10);
    pub const DETUNE: Self = Self(11);
    pub const PE_ATK: Self = Self(12);
    pub const PE_HOLD: Self = Self(13);
    pub const PE_REL: Self = Self(14);
    pub const PE_REST: Self = Self(15);
    pub const PL_RATE: Self = Self(16);
    pub const PL_DEPTH: Self = Self(17);
    pub const PL_SHAPE: Self = Self(18);
    pub const PL_DEST: Self = Self(19);
    pub const NZ_COLOR: Self = Self(20);
    pub const AC_EVERY: Self = Self(21);
    pub const NZ_PUMP: Self = Self(22);
    pub const BASSOCT: Self = Self(23);
    pub const DR_CUT: Self = Self(24);
    pub const DR_RES: Self = Self(25);
    pub const DE_TRIG: Self = Self(26);
    pub const DE_ATK: Self = Self(27);
    pub const DE_DEC: Self = Self(28);
    pub const DE_AMT: Self = Self(29);
    pub const DL_RATE: Self = Self(30);
    pub const DL_DEPTH: Self = Self(31);
    pub const DL_SHAPE: Self = Self(32);
    pub const DL_DEST: Self = Self(33);
    pub const AC_CUT: Self = Self(34);
    pub const AC_RES: Self = Self(35);
    pub const AC_ENV: Self = Self(36);
    pub const AC_DEC: Self = Self(37);
    pub const AC_ACC: Self = Self(38);
    pub const AC_SLIDE: Self = Self(39);
    pub const AC_WAVE: Self = Self(40);
    pub const AC_DENS: Self = Self(41);
    pub const AC_LEN: Self = Self(42);
    pub const AC_RANGE: Self = Self(43);
    pub const AC_MUT: Self = Self(44);
    pub const AC_OCT: Self = Self(45);
    pub const DRIVE: Self = Self(46);
    pub const SWING: Self = Self(47);
    pub const PATTERN: Self = Self(48);
    pub const LANE: Self = Self(49);
    pub const ECHO: Self = Self(67);
    pub const ETIME: Self = Self(68);
    pub const REVERB: Self = Self(69);
    pub const SIZE: Self = Self(70);
    pub const ROOT: Self = Self(71);
    pub const MODE: Self = Self(72);
    pub const TUNE: Self = Self(73);
    pub const MIDI: Self = Self(74);
    pub const THEME: Self = Self(75);
    pub const STYLE: Self = Self(76);
    pub const LINKS: Self = Self(77);
    pub const LK_KICK: Self = Self(78);
    pub const LK_SNARE: Self = Self(79);
    pub const LK_HAT: Self = Self(80);
    pub const LK_PERC: Self = Self(81);
    pub const OCT: Self = Self(82);
    pub const PAD_OCT: Self = Self(83);
    pub const DR_INST: Self = Self(84);
    pub const KIT_KICK: Self = Self(85);
    pub const KIT_SNARE: Self = Self(86);
    pub const KIT_HAT: Self = Self(87);
    pub const KIT_PERC: Self = Self(88);
    pub const PAD_MAN: Self = Self(89);
    pub const DR_MAN: Self = Self(90);
    pub const AC_MAN: Self = Self(91);
    pub const CHORD: Self = Self(92);
    pub const VOICING: Self = Self(93);
    pub const DD_CHANGE: Self = Self(94);
    pub const DD_EVERY: Self = Self(95);
    pub const DD_POOL: Self = Self(96);
    pub const DD_MUTATE: Self = Self(97);
    pub const CM_LVL: Self = Self(98);
    pub const CM_SND: Self = Self(99);
    pub const CM_DENS: Self = Self(100);
    pub const CM_OCT: Self = Self(101);
    pub const CM_DEC: Self = Self(102);
    pub const CM_STYLE: Self = Self(103);
    pub const CM_RATE: Self = Self(104);
    pub const CM_RANGE: Self = Self(105);
    pub const PROB: Self = Self(106);
    pub const PROB1: Self = Self(107);
    pub const PROB2: Self = Self(108);
    pub const PROB3: Self = Self(109);
    pub const PROB4: Self = Self(110);
    pub const PROB5: Self = Self(111);
    pub const PAN: Self = Self(112);
    pub const PAN1: Self = Self(113);
    pub const PAN2: Self = Self(114);
    pub const PAN3: Self = Self(115);
    pub const PAN4: Self = Self(116);
    pub const SEND: Self = Self(117);
    pub const SEND1: Self = Self(118);
    pub const SEND2: Self = Self(119);
    pub const SEND3: Self = Self(120);
    pub const SEND4: Self = Self(121);
    pub const CM_MAN: Self = Self(122);
    pub const RV_TYPE: Self = Self(123);
    pub const RV_DAMP: Self = Self(124);
    pub const RV_PRE: Self = Self(125);
    pub const RV_X: Self = Self(126);
    pub const PL_CHO: Self = Self(127);
    pub const MC_RATIO: Self = Self(128);
    pub const MC_THR: Self = Self(129);
    pub const MC_MAKE: Self = Self(130);
    pub const ML_CEIL: Self = Self(131);
    pub const LLEN: Self = Self(132);
    pub const LLEN1: Self = Self(133);
    pub const LLEN2: Self = Self(134);
    pub const LLEN3: Self = Self(135);
    pub const LLEN4: Self = Self(136);
    pub const LLEN5: Self = Self(137);
    pub const DENS: Self = Self(138);
    pub const DENS1: Self = Self(139);
    pub const DENS2: Self = Self(140);
    pub const DENS3: Self = Self(141);
    pub const DENS4: Self = Self(142);
    pub const DENS5: Self = Self(143);
    pub const LSRC: Self = Self(144);
    pub const LSRC1: Self = Self(145);
    pub const LSRC2: Self = Self(146);
    pub const LSRC3: Self = Self(147);
    pub const LSRC4: Self = Self(148);
    pub const LSRC5: Self = Self(149);
    pub const CH_LOOP: Self = Self(150);
    pub const AC_RESP: Self = Self(151);
    pub const BL_MODE: Self = Self(152);
    pub const BL_LEN: Self = Self(153);
    pub const BL_MOVE: Self = Self(154);
    pub const BL_GLIDE: Self = Self(155);
    pub const CM_VU: Self = Self(156);
    pub const CM_LOOP: Self = Self(157);
    pub const CM_LEAP: Self = Self(158);
    pub const AC_CHG: Self = Self(159);
    pub const PAD_COMP: Self = Self(160);
    pub const GHOST: Self = Self(161);
    pub const ACCENT: Self = Self(162);
    pub const HUMAN: Self = Self(163);
    pub const CHAIN: Self = Self(164);
    pub const SLICE: Self = Self(165);
    pub const TIME: Self = Self(166);
    pub const COND: Self = Self(167);
    pub const COND1: Self = Self(168);
    pub const COND2: Self = Self(169);
    pub const COND3: Self = Self(170);
    pub const COND4: Self = Self(171);
    pub const COND5: Self = Self(172);
    pub const AC_LOCK: Self = Self(173);
    pub const CM_MOTIF: Self = Self(174);
    pub const CM_LOCK: Self = Self(175);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamInfo {
    pub name: &'static str,
    pub min: i16,
    pub max: i16,
    pub default: i16,
}
impl ParamInfo {
    pub const fn contains(&self, value: i16) -> bool {
        value >= self.min && value <= self.max
    }
}
pub static PARAMETERS: [ParamInfo; PARAMETER_COUNT] = [
    ParamInfo {
        name: "Pad",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Acid",
        min: 0,
        max: 100,
        default: 55,
    },
    ParamInfo {
        name: "Bass",
        min: 0,
        max: 100,
        default: 65,
    },
    ParamInfo {
        name: "Drums",
        min: 0,
        max: 100,
        default: 75,
    },
    ParamInfo {
        name: "Vinyl",
        min: 0,
        max: 100,
        default: 20,
    },
    ParamInfo {
        name: "Tempo",
        min: 70,
        max: 190,
        default: 128,
    },
    ParamInfo {
        name: "Evolve",
        min: 0,
        max: 100,
        default: 40,
    },
    ParamInfo {
        name: "Motion",
        min: 0,
        max: 100,
        default: 60,
    },
    ParamInfo {
        name: "Drift",
        min: 0,
        max: 100,
        default: 60,
    },
    ParamInfo {
        name: "Bright",
        min: 0,
        max: 100,
        default: 35,
    },
    ParamInfo {
        name: "Sound",
        min: 0,
        max: 23,
        default: 1,
    },
    ParamInfo {
        name: "Detune",
        min: 0,
        max: 100,
        default: 30,
    },
    ParamInfo {
        name: "Attack",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Hold",
        min: 0,
        max: 100,
        default: 35,
    },
    ParamInfo {
        name: "Release",
        min: 0,
        max: 100,
        default: 55,
    },
    ParamInfo {
        name: "Rest",
        min: 0,
        max: 100,
        default: 20,
    },
    ParamInfo {
        name: "Rate",
        min: 0,
        max: 11,
        default: 4,
    },
    ParamInfo {
        name: "Depth",
        min: 0,
        max: 100,
        default: 25,
    },
    ParamInfo {
        name: "Shape",
        min: 0,
        max: 5,
        default: 0,
    },
    ParamInfo {
        name: "Target",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Wear",
        min: 0,
        max: 7,
        default: 1,
    },
    ParamInfo {
        name: "Every",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Pump",
        min: 0,
        max: 100,
        default: 60,
    },
    ParamInfo {
        name: "Octave",
        min: -3,
        max: 3,
        default: 0,
    },
    ParamInfo {
        name: "Cutoff",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Reso",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Trigger",
        min: 0,
        max: 5,
        default: 0,
    },
    ParamInfo {
        name: "Attack",
        min: 0,
        max: 100,
        default: 25,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 55,
    },
    ParamInfo {
        name: "Amount",
        min: 0,
        max: 100,
        default: 35,
    },
    ParamInfo {
        name: "Rate",
        min: 0,
        max: 11,
        default: 6,
    },
    ParamInfo {
        name: "Depth",
        min: 0,
        max: 100,
        default: 55,
    },
    ParamInfo {
        name: "Shape",
        min: 0,
        max: 5,
        default: 0,
    },
    ParamInfo {
        name: "Target",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Cutoff",
        min: 0,
        max: 100,
        default: 35,
    },
    ParamInfo {
        name: "Reso",
        min: 0,
        max: 100,
        default: 70,
    },
    ParamInfo {
        name: "Env mod",
        min: 0,
        max: 100,
        default: 55,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 40,
    },
    ParamInfo {
        name: "Accent",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Slide",
        min: 0,
        max: 100,
        default: 30,
    },
    ParamInfo {
        name: "Sound",
        min: 0,
        max: 30,
        default: 0,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 60,
    },
    ParamInfo {
        name: "Length",
        min: 1,
        max: 16,
        default: 16,
    },
    ParamInfo {
        name: "Range",
        min: 1,
        max: 3,
        default: 1,
    },
    ParamInfo {
        name: "Mutate",
        min: 0,
        max: 100,
        default: 25,
    },
    ParamInfo {
        name: "Acid oct",
        min: -3,
        max: 3,
        default: 0,
    },
    ParamInfo {
        name: "Drive",
        min: 0,
        max: 100,
        default: 40,
    },
    ParamInfo {
        name: "Swing",
        min: 0,
        max: 100,
        default: 0,
    },
    ParamInfo {
        name: "Pattern",
        min: 0,
        max: 5,
        default: 0,
    },
    ParamInfo {
        name: "Level",
        min: 0,
        max: 100,
        default: 85,
    },
    ParamInfo {
        name: "Tune",
        min: -50,
        max: 50,
        default: 0,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Level",
        min: 0,
        max: 100,
        default: 60,
    },
    ParamInfo {
        name: "Tune",
        min: -50,
        max: 50,
        default: 0,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Level",
        min: 0,
        max: 100,
        default: 65,
    },
    ParamInfo {
        name: "Tune",
        min: -50,
        max: 50,
        default: 0,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Level",
        min: 0,
        max: 100,
        default: 55,
    },
    ParamInfo {
        name: "Tune",
        min: -50,
        max: 50,
        default: 0,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 35,
    },
    ParamInfo {
        name: "Level",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Tune",
        min: -50,
        max: 50,
        default: 0,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Level",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Tune",
        min: -50,
        max: 50,
        default: 0,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 40,
    },
    ParamInfo {
        name: "Echo",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Echo time",
        min: 0,
        max: 5,
        default: 1,
    },
    ParamInfo {
        name: "Reverb",
        min: 0,
        max: 100,
        default: 40,
    },
    ParamInfo {
        name: "Size",
        min: 0,
        max: 100,
        default: 70,
    },
    ParamInfo {
        name: "Root",
        min: 0,
        max: 11,
        default: 9,
    },
    ParamInfo {
        name: "Mode",
        min: 0,
        max: 7,
        default: 5,
    },
    ParamInfo {
        name: "Tune",
        min: -50,
        max: 50,
        default: 0,
    },
    ParamInfo {
        name: "MIDI out",
        min: 0,
        max: 1,
        default: 1,
    },
    ParamInfo {
        name: "Theme",
        min: 0,
        max: 5,
        default: 0,
    },
    ParamInfo {
        name: "Sync",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Links",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Kick",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Snare",
        min: 0,
        max: 100,
        default: 40,
    },
    ParamInfo {
        name: "Hats",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Perc",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Song oct",
        min: -3,
        max: 3,
        default: 0,
    },
    ParamInfo {
        name: "Pad oct",
        min: -3,
        max: 3,
        default: 0,
    },
    ParamInfo {
        name: "Sound",
        min: 0,
        max: 23,
        default: 0,
    },
    ParamInfo {
        name: "Kick",
        min: 0,
        max: 13,
        default: 0,
    },
    ParamInfo {
        name: "Snare",
        min: 0,
        max: 13,
        default: 0,
    },
    ParamInfo {
        name: "Hats",
        min: 0,
        max: 13,
        default: 0,
    },
    ParamInfo {
        name: "Perc",
        min: 0,
        max: 13,
        default: 0,
    },
    ParamInfo {
        name: "Pad hand",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Bass hand",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Acid hand",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Chord",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Voicing",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Change",
        min: 0,
        max: 2,
        default: 0,
    },
    ParamInfo {
        name: "Every",
        min: 0,
        max: 5,
        default: 2,
    },
    ParamInfo {
        name: "Pool",
        min: 2,
        max: 6,
        default: 4,
    },
    ParamInfo {
        name: "Mutate",
        min: 0,
        max: 2,
        default: 0,
    },
    ParamInfo {
        name: "Pluck",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Sound",
        min: 0,
        max: 57,
        default: 1,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Pluck oct",
        min: -2,
        max: 2,
        default: 0,
    },
    ParamInfo {
        name: "Decay",
        min: 0,
        max: 100,
        default: 45,
    },
    ParamInfo {
        name: "Style",
        min: 0,
        max: 5,
        default: 0,
    },
    ParamInfo {
        name: "Rate",
        min: 0,
        max: 3,
        default: 2,
    },
    ParamInfo {
        name: "Range",
        min: 1,
        max: 3,
        default: 2,
    },
    ParamInfo {
        name: "Prob",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Prob",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Prob",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Prob",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Prob",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Prob",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Pan",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Pan",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Pan",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Pan",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Pan",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Send",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Send",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Send",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Send",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Send",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Pluck hand",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Type",
        min: 0,
        max: 2,
        default: 0,
    },
    ParamInfo {
        name: "Damping",
        min: 0,
        max: 100,
        default: 47,
    },
    ParamInfo {
        name: "Predelay",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Amount",
        min: 0,
        max: 100,
        default: 30,
    },
    ParamInfo {
        name: "Chorus",
        min: 0,
        max: 100,
        default: 0,
    },
    ParamInfo {
        name: "Ratio",
        min: 0,
        max: 6,
        default: 1,
    },
    ParamInfo {
        name: "Thresh",
        min: 0,
        max: 100,
        default: 60,
    },
    ParamInfo {
        name: "Make-up",
        min: 0,
        max: 100,
        default: 10,
    },
    ParamInfo {
        name: "Ceiling",
        min: 0,
        max: 100,
        default: 100,
    },
    ParamInfo {
        name: "Length",
        min: 1,
        max: 16,
        default: 16,
    },
    ParamInfo {
        name: "Length",
        min: 1,
        max: 16,
        default: 16,
    },
    ParamInfo {
        name: "Length",
        min: 1,
        max: 16,
        default: 16,
    },
    ParamInfo {
        name: "Length",
        min: 1,
        max: 16,
        default: 16,
    },
    ParamInfo {
        name: "Length",
        min: 1,
        max: 16,
        default: 16,
    },
    ParamInfo {
        name: "Length",
        min: 1,
        max: 16,
        default: 16,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Density",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Source",
        min: 0,
        max: 6,
        default: 0,
    },
    ParamInfo {
        name: "Source",
        min: 0,
        max: 6,
        default: 0,
    },
    ParamInfo {
        name: "Source",
        min: 0,
        max: 6,
        default: 0,
    },
    ParamInfo {
        name: "Source",
        min: 0,
        max: 6,
        default: 0,
    },
    ParamInfo {
        name: "Source",
        min: 0,
        max: 6,
        default: 0,
    },
    ParamInfo {
        name: "Source",
        min: 0,
        max: 6,
        default: 0,
    },
    ParamInfo {
        name: "Loop",
        min: 0,
        max: 6,
        default: 0,
    },
    ParamInfo {
        name: "Answer",
        min: 0,
        max: 44,
        default: 0,
    },
    ParamInfo {
        name: "Mode",
        min: 0,
        max: 16,
        default: 0,
    },
    ParamInfo {
        name: "Length",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Move",
        min: 0,
        max: 100,
        default: 30,
    },
    ParamInfo {
        name: "Glide",
        min: 0,
        max: 100,
        default: 0,
    },
    ParamInfo {
        name: "Deja vu",
        min: 0,
        max: 100,
        default: 0,
    },
    ParamInfo {
        name: "Loop",
        min: 0,
        max: 3,
        default: 2,
    },
    ParamInfo {
        name: "Leaps",
        min: 0,
        max: 100,
        default: 0,
    },
    ParamInfo {
        name: "Mutates",
        min: 0,
        max: 3,
        default: 0,
    },
    ParamInfo {
        name: "Comp",
        min: 0,
        max: 8,
        default: 0,
    },
    ParamInfo {
        name: "Ghosts",
        min: 0,
        max: 100,
        default: 0,
    },
    ParamInfo {
        name: "Accents",
        min: 0,
        max: 100,
        default: 50,
    },
    ParamInfo {
        name: "Humanize",
        min: 0,
        max: 100,
        default: 0,
    },
    ParamInfo {
        name: "Chain",
        min: 0,
        max: 9,
        default: 0,
    },
    ParamInfo {
        name: "Slice",
        min: 0,
        max: 9,
        default: 0,
    },
    ParamInfo {
        name: "Time",
        min: 0,
        max: 2,
        default: 0,
    },
    ParamInfo {
        name: "Cond",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Cond",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Cond",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Cond",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Cond",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Cond",
        min: 0,
        max: 7,
        default: 0,
    },
    ParamInfo {
        name: "Frozen",
        min: 0,
        max: 1,
        default: 0,
    },
    ParamInfo {
        name: "Motif",
        min: 0,
        max: 100,
        default: 60,
    },
    ParamInfo {
        name: "Frozen",
        min: 0,
        max: 1,
        default: 0,
    },
];
