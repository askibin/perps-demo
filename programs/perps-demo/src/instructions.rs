// admin instructions
pub mod add_pool;
pub mod add_token;
pub mod init;

// test instructions
pub mod set_test_oracle_price;

// public instructions
pub mod add_liquidity;
pub mod close_position;
pub mod liquidate;
pub mod open_position;
pub mod remove_liquidity;
pub mod swap;

// bring everything in scope
pub use add_pool::*;
pub use add_token::*;
pub use init::*;

pub use set_test_oracle_price::*;

pub use add_liquidity::*;
pub use close_position::*;
pub use liquidate::*;
pub use open_position::*;
pub use remove_liquidity::*;
pub use swap::*;
