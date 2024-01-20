use std::sync::Arc;

use druid::lens::Identity;
use druid::widget::{
    Button, Checkbox, Container, Controller, ControllerHost, Either, Flex, Label, List, TextBox,
};
use druid::{
    Color, Data, Env, EventCtx, LensExt, Menu, MenuItem, Point, UpdateCtx, Widget, WidgetExt,
};
use tracing::info;

use crate::gui::ui::{UIBrowser, UISettings, UISettingsRule, UIState, SAVE_RULE, SAVE_RULES};

pub(crate) fn rules_content(browsers: Arc<Vec<UIBrowser>>) -> impl Widget<UISettings> {
    info!("recreating rules_content");
    let browsers_arc = browsers.clone();

    let app_name_row: Label<UISettings> = Label::new("Rules");

    // TODO: add default_profile also to rules

    let rules_list = List::new(move || create_rule(&browsers_arc))
        .lens(UISettings::rules)
        .scroll()
        .vertical()
        .content_must_fill(true);

    // viewport size is fixed, while scrollable are is full size
    let rules_list = Container::new(rules_list).expand_height();

    let add_rule_button =
        Button::new("Add rule").on_click(move |ctx, data: &mut UISettings, _env| {
            let rule = data.add_empty_rule();
            ctx.submit_command(SAVE_RULE.with(rule.index))
        });

    let col = Flex::column()
        .with_child(app_name_row)
        .with_flex_child(rules_list, 1.0)
        .with_child(add_rule_button)
        .expand_height();

    return col;
}

fn create_rule(browsers: &Arc<Vec<UIBrowser>>) -> impl Widget<UISettingsRule> {
    info!("Recreating rule");
    let url_pattern_label = Label::new("If URL contains");
    let profile_label = Label::new("Open in");

    let remove_rule_button =
        Button::new("➖").on_click(move |ctx, data: &mut UISettingsRule, _env| {
            data.deleted = true;
            ctx.submit_command(SAVE_RULES.with(()));
        });

    let action_row = Flex::row().with_child(remove_rule_button);

    let text_box = TextBox::new()
        .with_placeholder("https://")
        .with_text_size(18.0);

    //let formatter = ParseFormatter::new();
    //let value_text_box = ValueTextBox::new(text_box, formatter).update_data_while_editing(true);
    let value_text_box = ControllerHost::new(text_box, SaveRulesOnDataChange);

    let url_pattern = value_text_box
        .fix_width(300.0)
        .lens(UISettingsRule::url_pattern);

    let browsers_clone = browsers.clone();
    let browsers_clone2 = browsers.clone();
    let selected_profile = Label::dynamic(move |rule: &UISettingsRule, _| {
        let browser_maybe = find_browser(&browsers_clone, rule.profile.clone());
        let profile_name_maybe = browser_maybe.map(|b| b.get_full_name());
        let profile_name = profile_name_maybe.unwrap_or("Unknown".to_string());

        format!("{profile_name} ▼")
    })
    .border(Color::rgba(0.5, 0.5, 0.5, 0.9), 0.5)
    .on_click(move |ctx: &mut EventCtx, rule: &mut UISettingsRule, _env| {
        // Windows requires exact position relative to the window
        /*let position = Point::new(
            window_size.width
                - crate::gui::ui::PADDING_X
                - crate::gui::ui::OPTIONS_LABEL_SIZE / 2.0,
            window_size.height
                - crate::gui::ui::PADDING_Y
                - crate::gui::ui::OPTIONS_LABEL_SIZE / 2.0,
        );*/

        let rule_index = rule.index.clone();
        let menu = make_profiles_menu(browsers_clone2.clone(), rule_index);
        ctx.show_context_menu(menu, Point::new(0.0, 0.0));
    });

    let url_pattern_row = Flex::row()
        .with_child(url_pattern_label)
        .with_child(url_pattern);

    let browsers_clone3 = browsers.clone();

    let incognito_either = Either::new(
        move |rule: &UISettingsRule, _env| {
            let browser_maybe = find_browser(&browsers_clone3, rule.profile.clone());
            let browser_supports_incognito_maybe = browser_maybe.map(|p| p.supports_incognito);
            let profile_supports_incognito = browser_supports_incognito_maybe.unwrap_or(false);
            profile_supports_incognito
        },
        {
            let incognito_checkbox =
                ControllerHost::new(Checkbox::new("incognito"), SaveRulesOnDataChange)
                    .lens(UISettingsRule::incognito);
            incognito_checkbox
        },
        Flex::column(),
    );

    let profile_row = Flex::row()
        .with_child(profile_label)
        .with_child(selected_profile)
        .with_child(incognito_either);

    return Either::new(|data: &UISettingsRule, _env| data.deleted, Flex::column(), {
        Container::new(
            Flex::column()
                .with_child(action_row)
                .with_child(url_pattern_row)
                .with_child(profile_row),
        )
        .padding(10.0)
        .background(Color::rgba(0.1, 0.1, 0.1, 0.9))
        .rounded(10.0)
        .border(Color::rgba(0.5, 0.5, 0.5, 0.9), 0.5)
        .padding(10.0)
    });
}
fn find_browser(browsers: &Arc<Vec<UIBrowser>>, unique_id: String) -> Option<&UIBrowser> {
    let option = browsers.iter().filter(|b| b.unique_id == unique_id).next();
    return option;
}

struct SaveRulesOnDataChange;

impl<T: Data, W: Widget<T>> Controller<T, W> for SaveRulesOnDataChange {
    fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        child.update(ctx, old_data, data, env);
        if !old_data.same(data) {
            ctx.submit_command(SAVE_RULES.with(()));
        }
    }
}

fn make_profiles_menu(browsers: Arc<Vec<UIBrowser>>, rule_index: usize) -> Menu<UIState> {
    // TODO: should also add Prompt (no profile) as an option (also in settings).
    let menu = browsers
        .iter()
        .map(|b| {
            let profile_full_name = b.get_full_name();
            let profile_id = b.unique_id.clone();
            let profile_id_clone = profile_id.clone();

            MenuItem::new(profile_full_name)
                .selected_if(move |data: &UISettingsRule, _env| data.profile == profile_id)
                .on_activate(move |ctx, data: &mut UISettingsRule, _env| {
                    data.profile = profile_id_clone.clone();
                    ctx.submit_command(SAVE_RULE.with(rule_index))
                })
                .lens(
                    UIState::ui_settings
                        .then(UISettings::rules)
                        .then(Identity.index(rule_index).in_arc()),
                )
        })
        .fold(Menu::empty(), |acc, e| acc.entry(e));

    menu
}