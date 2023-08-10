use solana_program::native_token::LAMPORTS_PER_SOL;

pub const MS_IN_SEC: i64 = 1000;
pub const ONE_MINUTE_IN_MSEC: i64 = 60 * MS_IN_SEC;
pub const THIRTY_MINUTES_IN_MSEC: i64 = 30 * ONE_MINUTE_IN_MSEC;

pub const ONE_SOL: u64 = LAMPORTS_PER_SOL;
pub const THREE_SOL: u64 = 3 * ONE_SOL;
pub const TEN_SOL: u64 = 10 * ONE_SOL;