use serde::{Deserialize, Serialize};

use crate::{cfg::AnalysisConfig, unit::Db, util::RingBuffer};

use super::{AudibleSpec, fft::FftData};

pub struct HpsData {
    pub past_db: AudibleSpec<RingBuffer<Db>>,
    pub h_enhanced: AudibleSpec<Db>,
    pub p_enhanced: AudibleSpec<Db>,
}

fn median(iter: impl ExactSizeIterator<Item = &Db>) -> Db {
    let len = iter.len();
    **iter
        .collect::<Vec<_>>()
        .select_nth_unstable_by(len / 2, |a, b| a.partial_cmp(b).unwrap())
        .1
}

impl HpsData {
    pub fn blank(cfg: &AnalysisConfig) -> Self {
        Self {
            past_db: AudibleSpec::blank_clone(
                &RingBuffer::from_default(cfg.hps.h_filter_span),
                cfg,
            ),
            h_enhanced: AudibleSpec::blank_default(cfg),
            p_enhanced: AudibleSpec::blank_default(cfg),
        }
    }

    pub fn advance(mut self, cfg: &AnalysisConfig, fft: &FftData) -> Self {
        let hps = &cfg.hps;

        for (i, buf) in self.past_db.iter_mut().enumerate() {
            buf.replace(fft.db[i]);
        }

        self.h_enhanced = AudibleSpec(self.past_db.iter().map(|buf| median(buf.iter())).collect());
        let mut p_enhanced = Vec::with_capacity(cfg.max_aidx());

        for i in 0..cfg.max_aidx() {
            let above = hps.p_filter_span / 2;
            let below = hps.p_filter_span - above - 1;
            let range;
            if i < below {
                range = 0..hps.p_filter_span;
            } else if i > cfg.max_aidx() - hps.p_filter_span {
                range = (cfg.max_aidx() - hps.p_filter_span)..cfg.max_aidx();
            } else {
                range = (i - below)..(i + above);
            }
            p_enhanced.push(median(fft.db[range].iter()));
        }

        self
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct HpsConfig {
    pub p_filter_span: usize,
    pub h_filter_span: usize,
}

impl Default for HpsConfig {
    fn default() -> Self {
        Self {
            // 44.1k samples/s * 0.2s / 1024 samples/hop
            //  = 8.61328125
            p_filter_span: 9,
            // (max_idx / max_freq) idx/hz * 500`hz
            //  = ((8000 / 44_100 * 4096) / 8000) * 500
            //  = 46.43990929705215 indices
            h_filter_span: 46,
        }
    }
}
