// export modules
pub mod add_stickers;
pub mod cancel;
pub mod common;
pub mod deleted_sets_upd;
pub mod my_stickers;
pub mod source;
pub mod start;
pub mod steal_sticker_set;

// export functions from modules so as not to bother with the functions paths
pub use add_stickers::{
    add_stickers_handler, add_stickers_to_user_owned_sticker_set, get_stickers_to_add,
    get_stolen_sticker_set,
};
pub use cancel::cancel_handler;
pub use common::{add_stickers, process_non_sticker};
pub use deleted_sets_upd::deleted_sets_upd;
pub use my_stickers::{my_stickers_handler, process_button};
pub use source::source_handler;
pub use start::start_handler;
pub use steal_sticker_set::{
    create_new_sticker_set, get_sticker_set_name, steal_sticker_set_handler,
};
