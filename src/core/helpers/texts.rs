//! Its bad to storing messages here, but i dont care

use telers::utils::text::{html_bold, html_code, html_quote, html_text_link};

use super::{common::get_page_begin_and_end, constants::TELEGRAM_STICKER_SET_URL};
use crate::domain::entities::set::Set;

pub fn sticker_set_message(
    sticker_set_title: &str,
    sticker_set_name: &str,
    sticker_set_link: &str,
) -> String {
    format!(
        "
        Now you have your own sticker pack {new_ss_url}. \
        If you want, you can add any sticker(s) to this pack using command /addstickers. \
        Also you can manage your new sticker pack, using official Telegram bot @Stickers, and to do it, \
        you should use this internal name of your sticker pack: {sticker_set_name}
        ",
        new_ss_url = html_text_link(html_quote(sticker_set_title), sticker_set_link),
        sticker_set_name = html_code(sticker_set_name)
    )
}

pub fn start_message(username: &str) -> String {
    format!(
        "
    Hello, {username}! This is bot to steal stickers!\n\
    List of commands you can use:\n\n\
    /stealpack - Steal sticker pack\n\
    /addstickers - Add sticker to a sticker pack stolen by this bot\n\
    /mystickers - List of your stolen stickers\n\
    /stats - See the bot statistics\n\n\
    /help - Show this message\n\
    /cancel - Cancel last command\n\
    /source or /src - Show source code of the bot\n\
        ",
    )
}

pub fn current_page_message(
    current_page: usize,
    pages_number: u32,
    sets_number_per_page: usize,
    list: &[Set],
) -> String {
    let (begin_page_index, end_page_index) =
        get_page_begin_and_end(current_page, pages_number, list.len(), sets_number_per_page);

    let mut sticker_sets_page = format!("List of your stickers ({current_page} page):\n");
    for set in list.iter().take(end_page_index).skip(begin_page_index) {
        let sticker_set_name = set.short_name.as_str();
        let sticker_set_title = set.title.as_str();
        let sticker_set_link = format!("{TELEGRAM_STICKER_SET_URL}{sticker_set_name}");
        let sticker_set = html_text_link(html_quote(sticker_set_title), sticker_set_link);

        sticker_sets_page.push_str(&sticker_set);
        if list.len() != 1 {
            sticker_sets_page.push_str(" | ");
        }
    }

    sticker_sets_page
}

pub fn personal_stats_message(all_count: i64, not_deleted_count: i64) -> String {
    format!(
        "
    {personal_statistics_text}\n\n\
    The current number of stolen stickers: {not_deleted_count_code}\n\
    Total number of stolen sticker packs (including {deleted_count_code} deleted ones): {all_count_code}
        ",
        personal_statistics_text = html_bold("Personal statistics"),
        not_deleted_count_code = html_code(not_deleted_count.to_string()),
        deleted_count_code = html_code((all_count - not_deleted_count).to_string()),
        all_count_code = html_code(all_count.to_string())
    )
}

#[test]
fn current_page_message_test() {
    let mut list = Vec::new();
    for i in 0..5 {
        list.push(Set {
            tg_id: i,
            short_name: format!("short_name{i}"),
            deleted: false,
            title: format!("title{i}"),
        });
    }

    let message = current_page_message(1, 1, 50, &list);

    assert_eq!(
        message.as_str(),
        "List of your stickers (1 page):\n\
        <a href=\"t.me/addstickers/short_name0\">title0</a> \
        | <a href=\"t.me/addstickers/short_name1\">title1</a> \
        | <a href=\"t.me/addstickers/short_name2\">title2</a> \
        | <a href=\"t.me/addstickers/short_name3\">title3</a> \
        | <a href=\"t.me/addstickers/short_name4\">title4</a> \
        | "
    );
}
