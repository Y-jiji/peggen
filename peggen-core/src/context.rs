use crate::*;

const MEMO_SIZE: usize = 256;

pub struct ParseContext {
    pub trace: Vec<usize>,
    pub tags: Vec<Tag>,
    memo: [(u64, i64); MEMO_SIZE],
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            trace: Vec::new(),
            tags: Vec::new(),
            memo: [(u64::MAX, 0); MEMO_SIZE],
        }
    }

    pub fn clear(&mut self) {
        self.trace.clear();
        self.tags.clear();
        for slot in &mut self.memo {
            *slot = (u64::MAX, 0);
        }
    }

    #[inline(always)]
    pub fn memo_check(&self, pos: usize, rule_id: usize) -> Option<Result<usize, ()>> {
        let key = (pos as u64) ^ ((rule_id as u64) << 32);
        let idx = (key as usize) & (MEMO_SIZE - 1);
        let (stored_key, stored_val) = self.memo[idx];
        if stored_key == key {
            if stored_val < 0 {
                Some(Err(()))
            } else {
                Some(Ok(stored_val as usize))
            }
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn memo_store(&mut self, pos: usize, rule_id: usize, result: Result<usize, ()>) {
        let key = (pos as u64) ^ ((rule_id as u64) << 32);
        let idx = (key as usize) & (MEMO_SIZE - 1);
        let val = match result {
            Ok(end) => end as i64,
            Err(()) => -1,
        };
        self.memo[idx] = (key, val);
    }
}
