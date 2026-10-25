//! Its bad to storing messages here, but i dont care

use telers::utils::text::{html_blockquote, html_bold, html_code, html_quote, html_text_link};

use super::{common::get_page_begin_and_end, constants::TELEGRAM_STICKER_SET_URL};
use crate::{
    core::helpers::{
        constants::MAX_STICKER_SET_LENGTH,
        stats::{GlobalStats, PersonalStats},
    },
    domain::entities::set::Set,
};

pub fn ask_for_sticker_for_owner() -> &'static str {
    "Send me any sticker and I'll show you the owner ID of its sticker pack:"
}

pub fn sticker_without_pack_for_owner() -> &'static str {
    "Unable to retrieve the owner's ID because this sticker isn't in any sticker pack. \
     Try sending another sticker or use /cancel instead."
}

pub fn pack_owner_id(owner_id: i64) -> String {
    format!(
        "The owner ID of this sticker pack is {id}. Send another sticker or use /cancel instead.",
        id = html_code(owner_id),
    )
}

pub fn default_error_message() -> &'static str {
    "Sorry, an unexpected error has occurred."
}

pub fn now_enter_set_name() -> &'static str {
    "Now enter a name for your new sticker pack (1-64 characters). If you want, use /cancel to abort the command."
}

pub fn ask_for_sticker_or_link() -> &'static str {
    "Send me a sticker or a sticker pack link and I'll steal this sticker pack for you:"
}

pub fn set_title_too_long(max_len: usize) -> String {
    format!(
        "Too long name! Please enter a name up to {max_len} characters long:",
        max_len = html_code(max_len),
    )
}

pub fn set_title_too_short(min_len: usize, max_len: usize) -> String {
    format!(
        "Too short name! Please enter a name between {min_len} and {max_len} characters long:",
        min_len = html_code(min_len),
        max_len = html_code(max_len),
    )
}

pub fn stealing_pack_started(title: &str, one_go_limit: usize) -> String {
    format!(
        "Stealing sticker pack with name {title} for you... \
        It may take a few minutes if this sticker pack contains more than {one_go_limit} stickers.",
        title = html_code(html_quote(title)),
        one_go_limit = html_code(one_go_limit),
    )
}

pub fn do_you_want_to_steal(set_name: &str) -> String {
    format!(
        "Do you want to steal this {set}?",
        set = html_text_link(
            html_quote("sticker pack"),
            format!("{TELEGRAM_STICKER_SET_URL}{set_name}")
        )
    )
}

pub fn you_can_steal_at_any_time() -> &'static str {
    "Well, you can steal this and any other sticker set at any time."
}

pub fn invalid_set() -> &'static str {
    "This sticker set is invalid. Try to send another link or sticker."
}

pub fn link_or_sticker() -> &'static str {
    "Please send a valid sticker pack link or a sticker."
}

pub fn sticker_without_pack() -> &'static str {
    "This sticker doesn't belong to any sticker pack, or that pack has been deleted. Try sending another sticker or link."
}

pub fn user_doesnt_has_stickers_yet() -> &'static str {
    "You don't have a single stolen sticker pack. \
    Steal at least one sticker pack using /stealpack, and then try this command again."
}

pub fn ask_stolen_sticker_or_link() -> &'static str {
    "Send me a sticker or a link to one of your stolen sticker packs. \
    You can list them with /mystickers."
}

pub fn pack_not_stolen_by_bot() -> &'static str {
    "This sticker pack wasn't stolen by this bot, so I can't add stickers to it — \
    Telegram only allows modifying packs the bot created. \
    Use /mystickers to list your stolen packs, or /stealpack to steal this one."
}

pub fn pack_deleted() -> &'static str {
    "It looks like this sticker pack has been deleted. Try sending another one."
}

pub fn not_pack_owner() -> &'static str {
    "You're not the owner of this sticker pack. \
    Send me a pack that this bot stole for you, or use /stealpack to steal this one."
}

pub fn pack_current_length(current: usize, remaining: usize) -> String {
    format!(
        "This sticker pack currently has {current} stickers. You can add {remaining} more.",
        current = html_code(current),
        remaining = html_code(remaining),
    )
}

pub fn pack_full() -> &'static str {
    "This sticker pack is full. \
    Remove a few stickers and then try /addstickers again."
}

pub fn sticker_without_emoji() -> &'static str {
    "This sticker has no emoji, so it can't be added. Try sending another one."
}

pub fn sticker_limit_reached() -> String {
    format!(
        "You've reached the limit of {max_len} stickers. \
        Use /done to add the selected ones, or /undo to remove the latest. \
        Any further stickers will be ignored.",
        max_len = html_code(MAX_STICKER_SET_LENGTH),
    )
}

pub fn sticker_processed() -> &'static str {
    "Sticker added to the list. Send another, or use /done or /undo."
}

pub fn ask_for_stickers_to_add() -> &'static str {
    "Now send me the stickers you want to add to your pack. \
    When you're ready, use /done to add them all. \
    You can remove the last sticker with /undo, or cancel this command with /cancel."
}

pub fn sticker_removed_from_list() -> &'static str {
    "Sticker removed from the list. You can now use /done or /undo again."
}

pub fn nothing_to_remove() -> &'static str {
    "Nothing to remove. If you want, use /cancel to abort the command."
}

pub fn all_stickers_removed_undo() -> &'static str {
    "You've removed all stickers from the list. \
    Send at least one sticker before using /undo, or use /cancel to abort the command."
}

pub fn all_stickers_removed_done() -> &'static str {
    "You've removed all stickers. Send some stickers and then use /done, or use /cancel to abort the command."
}

pub fn no_stickers_sent_done() -> &'static str {
    "You haven't sent any stickers yet. Send some, then use /done. If you want, use /cancel to abort the command."
}

pub fn adding_stickers(count: usize, sticker_set_name: &str) -> String {
    let sticker_word = if count == 1 { "sticker" } else { "stickers" };

    format!(
        "Adding {count} {sticker_word} to {your} pack.. \
         This may take a few minutes if you selected a lot of stickers.",
        count = count,
        sticker_word = sticker_word,
        your = html_text_link("your", sticker_set_name),
    )
}

pub fn detailed_error_message(err: &str) -> String {
    format!(
        "Sorry, an unexpected error has occurred. Please contact the bot owner and provide this error:\n{}",
        html_blockquote(err)
    )
}

pub fn added_stickers_message(set_name: &str, set_title: &str) -> String {
    format!(
        "Sticker(s) have been added to {set}.",
        set = html_text_link(
            html_quote(set_title),
            format!("{TELEGRAM_STICKER_SET_URL}{set_name}")
        )
    )
}

pub fn stolen_set_message(
    sticker_set_title: &str,
    sticker_set_name: &str,
    sticker_set_link: &str,
) -> String {
    format!(
        "Now you have your own sticker pack {new_ss_url}. \
        If you want, you can add stickers to it using /addstickers. \
        You can also manage it with the official @Stickers bot — \
        just use the short name of your pack: {sticker_set_name}",
        new_ss_url = html_text_link(html_quote(sticker_set_title), sticker_set_link),
        sticker_set_name = html_code(sticker_set_name)
    )
}

pub fn start_message(username: &str) -> String {
    format!(
        "Hi, {username}! I can steal any sticker pack for you. \
        Just send me a sticker or a link to a sticker pack and I'll make it yours. \
        Use /mystickers to see all your stolen packs. \
        If you want to add new stickers to a pack you've stolen, use /addstickers. \
        Use /help to see the full list of commands.",
    )
}

pub fn help_message() -> &'static str {
    "List of commands:\n\n\
    /stealpack — steal a sticker pack\n\
    /addstickers — add stickers to a pack you've stolen\n\
    /mystickers — list your stolen packs\n\
    /stats — see bot statistics\n\
    /getowner — get the owner ID of a sticker pack\n\
    /cancel — cancel the current command\n\
    /source or /src — show the bot's source code\n\
    /help — show this message"
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
    for (i, set) in list
        .iter()
        .enumerate()
        .take(end_page_index)
        .skip(begin_page_index)
    {
        let i = i - begin_page_index;

        if i != 0 {
            sticker_sets_page.push_str(" | ");
        }

        let sticker_set_name = set.short_name.as_str();
        let sticker_set_title = set.title.as_str();

        let sticker_set_link = format!("{TELEGRAM_STICKER_SET_URL}{sticker_set_name}");
        let sticker_set = html_text_link(html_quote(sticker_set_title), sticker_set_link);

        sticker_sets_page.push_str(&sticker_set);
    }

    sticker_sets_page
}

pub fn personal_stats_message(personal_stats: PersonalStats) -> String {
    if personal_stats.total_user_sets_count == personal_stats.not_deleted_user_sets_count {
        format!(
            "{personal_statistics_text}\n\n\
            Stolen packs: {total_count}\n\
            You're ahead of {percent} of users.",
            personal_statistics_text = html_bold("Personal statistics"),
            total_count = html_code(personal_stats.total_user_sets_count),
            percent = html_code(personal_stats.better_than.to_string() + "%")
        )
    } else {
        format!(
            "{personal_statistics_text}\n\n\
            Stolen packs: {total_count}\n\
            Active: {active_count}\n\n\
            You're ahead of {percent} of users.",
            personal_statistics_text = html_bold("Personal statistics"),
            total_count = html_code(personal_stats.total_user_sets_count),
            active_count = html_code(personal_stats.not_deleted_user_sets_count),
            percent = html_code(personal_stats.better_than.to_string() + "%")
        )
    }
}

pub fn global_stats_message(global_stats: GlobalStats) -> String {
    format!(
        "{global_statistics_text}\n\n\
        Total sticker packs stolen: {total_stolen}\n\
        Total users: {total_users}\n
        ",
        global_statistics_text = html_bold("Global statistics"),
        total_stolen = html_code(global_stats.total_stolen),
        total_users = html_code(global_stats.total_users)
    )
}

pub fn pack_owned_by_user_and_bot(set_name: &str) -> String {
    format!(
        "This {set} belongs to you and was created by this bot. Do you want to add stickers to it?",
        set = html_text_link(
            html_quote("sticker pack"),
            format!("{TELEGRAM_STICKER_SET_URL}{set_name}")
        )
    )
}

pub fn pack_owned_by_user_not_bot(set_name: &str) -> String {
    format!(
        "It seems this is your {set}, but it wasn't stolen by this bot. Do you want to steal it?",
        set = html_text_link(
            html_quote("sticker pack"),
            format!("{TELEGRAM_STICKER_SET_URL}{set_name}")
        )
    )
}

pub fn add_stickers_cancelled() -> &'static str {
    "Okay, do whatever you want with it."
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
        | <a href=\"t.me/addstickers/short_name4\">title4</a>"
    );
}
