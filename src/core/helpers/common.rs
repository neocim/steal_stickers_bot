use random_string::generate;
use telers::types::Sticker;

use super::constants::TELEGRAM_STICKER_SET_URL;

/// Return sticker format for each sticker.
pub fn sticker_format(sticker: &Sticker) -> String {
    if sticker.is_animated {
        "animated".to_owned()
    } else if sticker.is_video {
        "video".to_owned()
    } else {
        "static".to_owned()
    }
}

/// Generate new random sticker set name. This function assumes that the `length` field is a **positive integer greater than 2**!
/// Otherwise errors may occur using the generated name.
pub fn generate_sticker_set_name_and_link(length: usize, bot_username: &str) -> (String, String) {
    let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
    let charset_without_nums = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

    let set_name_prefix = generate(1, charset_without_nums);
    let set_name = generate(length - 1, charset);
    let set_name = format!("{set_name_prefix}{set_name}_by_{bot_username}");

    let set_link = format!("{TELEGRAM_STICKER_SET_URL}{}", set_name);

    (set_name, set_link)
}

/// Return begin and end of specify page, using the necessary information
/// (more about the pages, etc. [here](../../bot_commands/handlers/my_stickers.rs)).
pub fn get_page_begin_and_end(
    current_page: usize,
    pages_number: u32,
    list_len: usize,
    sticker_sets_number_per_page: usize,
) -> (usize, usize) {
    let begin_page_index = sticker_sets_number_per_page * (current_page - 1);

    let end_page_index = if current_page == pages_number as usize {
        list_len
    } else {
        begin_page_index + sticker_sets_number_per_page
    };

    (begin_page_index, end_page_index)
}

/// If `<set_name>` ends with `by_<bot_username>`, return true, otherwise return false
pub fn set_created_by(set_name: &str, bot_username: &str) -> bool {
    if set_name.ends_with(format!("by_{bot_username}").as_str()) {
        return true;
    }

    false
}

#[test]
fn sticker_format_test() {
    let (generated_name, generated_link) = generate_sticker_set_name_and_link(15, "your_bot");

    assert!(!generated_name.contains("t.me/addstickers/"));
    assert!(
        generated_link.contains("t.me/addstickers/") && generated_link.contains(&generated_name)
    );

    assert_eq!(
        generated_link.len(),
        generated_name.len() + "t.me/addstickers/".len()
    )
}

#[test]
fn get_page_begin_and_end_test() {
    let (begin_index, end_index) = get_page_begin_and_end(1, 2, 97, 50);

    assert_eq!(begin_index, 0);
    assert_eq!(end_index, 50);

    let (begin_index, end_index) = get_page_begin_and_end(4, 4, 176, 50);

    assert_eq!(begin_index, 150);
    assert_eq!(end_index, 176);
}
