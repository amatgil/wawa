mod backend;
mod handle_raw_pad_links;
mod handlers;
mod highlighting;
mod uiuaizing;

use std::sync::LazyLock;

pub use handle_raw_pad_links::*;
pub use handlers::*;
pub use highlighting::*;
pub use uiuaizing::*;

pub static SELF_HANDLE: LazyLock<String> =
    LazyLock::new(|| dotenv::var("BOT_SELF_HANDLE").unwrap_or_else(|_| "wawa#0280".into()));
pub static SELF_ID: LazyLock<u64> =
    LazyLock::new(|| match dotenv::var("BOT_SELF_ID").map(|str| str.parse()) {
        Ok(Ok(id)) => id,
        _ => 1295816766446108795,
    });
