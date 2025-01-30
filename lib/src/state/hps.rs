use crate::{unit::Db, util::RingBuffer};

use super::AudibleSpec;

pub struct HpsData {
    pub past_db: AudibleSpec<RingBuffer<Db>>,
    pub h_enhanced: AudibleSpec<Db>,
    pub p_enhanced: AudibleSpec<Db>,
}