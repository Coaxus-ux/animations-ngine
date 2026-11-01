use px_ui::{Menu, MenuItem, Rect, Ui, menu_bar};

use super::{Context, menus_file, menus_palette};
use crate::app::neighbour_tab;
use crate::app::{CodeMode, Intent, PanelKind};
use crate::assets::ThemeChoice;

#[derive(Clone, Copy, Debug)]
pub struct MenuState {
    pub theme: ThemeChoice,
    pub scale: Option<i32>,
}

pub(super) type Entry<'a> = (MenuItem<'a>, Option<Intent>);

pub(super) fn entry<'a>(label: &'a str, shortcut: &'a str, intent: Option<Intent>) -> Entry<'a> {
    (
        MenuItem::new(label, shortcut).enabled(intent.is_some()),
        intent,
    )
}

pub(super) fn check<'a>(
    label: &'a str,
    shortcut: &'a str,
    on: bool,
    intent: Option<Intent>,
) -> Entry<'a> {
    let (item, intent) = entry(label, shortcut, intent);
    (item.checked(on), intent)
}

pub(super) const fn separator<'a>() -> Entry<'a> {
    (MenuItem::separator(), None)
}

pub(super) fn menus(ui: &mut Ui<'_>, context: &mut Context<'_>, area: Rect) {
    let recent: Vec<String> = context
        .recent
        .iter()
        .map(|path| {
            format!(
                "Open {}",
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned()
                )
            )
        })
        .collect();
    let menus = [
        ("File", menus_file::file(context, &recent)),
        ("Edit", edit(context)),
        ("Sprite", menus_file::sprite(context)),
        ("Layer", layer(context)),
        ("Frame", frame(context)),
        ("Palette", menus_palette::palette(context)),
        ("View", view(context)),
        ("Window", window(context)),
        (
            "Help",
            vec![entry(
                "Shortcuts and how-to",
                "F1",
                Some(Intent::ToggleHelp),
            )],
        ),
    ];
    let shown: Vec<Menu<'_>> = menus
        .iter()
        .map(|(title, entries)| Menu {
            title,
            items: entries.iter().map(|(item, _)| *item).collect(),
        })
        .collect();
    if let Some((menu, item)) = menu_bar(ui, area, &shown)
        && let Some(intent) = menus[menu].1[item].1.clone()
    {
        context.intents.push(intent);
    }
}

fn edit<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let history = context.document.map(crate::domain::Document::history);
    let open = context.document.is_some();
    let can = |yes: bool, intent: Intent| yes.then_some(intent);
    vec![
        entry(
            "Undo",
            "Ctrl+Z",
            can(
                history.is_some_and(crate::domain::History::can_undo),
                Intent::Undo,
            ),
        ),
        entry(
            "Redo",
            "Ctrl+Shift+Z",
            can(
                history.is_some_and(crate::domain::History::can_redo),
                Intent::Redo,
            ),
        ),
        separator(),
        entry("Cut", "Ctrl+X", can(open, Intent::Cut)),
        entry("Copy", "Ctrl+C", can(open, Intent::Copy)),
        entry(
            "Paste",
            "Ctrl+V",
            context.editing.clipboard.as_ref().map(|_| Intent::Paste),
        ),
        entry("Delete pixels", "Del", can(open, Intent::DeleteSelection)),
        separator(),
        entry("Select all", "Ctrl+A", can(open, Intent::SelectAll)),
        entry("Deselect", "Ctrl+D", can(open, Intent::Deselect)),
        separator(),
        entry(
            "Flip horizontally",
            "H",
            can(open, Intent::Flip { horizontal: true }),
        ),
        entry(
            "Flip vertically",
            "Shift+H",
            can(open, Intent::Flip { horizontal: false }),
        ),
    ]
}

fn layer<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let count = context
        .document
        .map_or(0, |document| document.layers().len());
    let active = context.view.layer.filter(|index| *index < count);
    let name = active
        .and_then(|index| context.document?.layers().get(index))
        .map(|layer| layer.name.clone());
    let hidden = name
        .as_ref()
        .is_some_and(|name| context.view.hidden.contains(name));
    let locked = name
        .as_ref()
        .is_some_and(|name| context.editing.locked.contains(name));
    let up = active.filter(|index| index + 1 < count);
    let down = active.filter(|index| *index > 0);
    vec![
        entry(
            "New pixel layer",
            "",
            context.document.map(|_| Intent::AddLayer),
        ),
        entry("Rename", "double click", active.map(Intent::StartRename)),
        entry("Delete", "", active.map(Intent::DeleteLayer)),
        entry(
            "Move up",
            "",
            up.map(|from| Intent::MoveLayer { from, to: from + 1 }),
        ),
        entry(
            "Move down",
            "",
            down.map(|from| Intent::MoveLayer { from, to: from - 1 }),
        ),
        separator(),
        check(
            "Hidden in this window",
            "",
            hidden,
            name.clone().map(Intent::ToggleHidden),
        ),
        check("Locked", "", locked, name.map(Intent::ToggleLocked)),
    ]
}

fn frame<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let open = |intent: Intent| context.document.map(|_| intent);
    vec![
        check(
            "Play",
            "Enter",
            context.editing.playing,
            open(Intent::TogglePlay),
        ),
        entry("Previous frame", ",", open(Intent::StepFrame(-1))),
        entry("Next frame", ".", open(Intent::StepFrame(1))),
        separator(),
        entry("New cel here", "", open(Intent::NewCel)),
        entry("Delete cel here", "", open(Intent::DeleteCel)),
        separator(),
        entry("Add a frame", "", open(Intent::AddFrame)),
        entry("Remove the last frame", "", open(Intent::DeleteFrame)),
        separator(),
        check(
            "Onion skin",
            "",
            context.view.onion,
            open(Intent::ToggleOnion),
        ),
    ]
}

fn view<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let at = context.raw.pointer.unwrap_or_default();
    let code = context.view.code;
    let theme = context.menu.theme;
    let scale = context.menu.scale;
    vec![
        entry("Zoom in", "+", Some(Intent::Zoom { steps: 1, at })),
        entry("Zoom out", "-", Some(Intent::Zoom { steps: -1, at })),
        entry("Fit to window", "0", Some(Intent::ZoomFit)),
        check(
            "Pixel grid",
            "",
            context.view.grid,
            Some(Intent::ToggleGrid),
        ),
        separator(),
        check(
            "Canvas only",
            "F2",
            code == CodeMode::Hidden,
            Some(Intent::SetCode(CodeMode::Hidden)),
        ),
        check(
            "Canvas and code",
            "F2",
            code == CodeMode::Split,
            Some(Intent::SetCode(CodeMode::Split)),
        ),
        check(
            "Code only",
            "F2",
            code == CodeMode::Only,
            Some(Intent::SetCode(CodeMode::Only)),
        ),
        separator(),
        check(
            "Dark theme",
            "",
            theme == ThemeChoice::Dark,
            Some(Intent::SetTheme(ThemeChoice::Dark)),
        ),
        check(
            "Light theme",
            "",
            theme == ThemeChoice::Light,
            Some(Intent::SetTheme(ThemeChoice::Light)),
        ),
        separator(),
        check(
            "UI size: automatic",
            "",
            scale.is_none(),
            Some(Intent::SetScale(None)),
        ),
        check(
            "UI size: small",
            "",
            scale == Some(1),
            Some(Intent::SetScale(Some(1))),
        ),
        check(
            "UI size: medium",
            "",
            scale == Some(2),
            Some(Intent::SetScale(Some(2))),
        ),
        check(
            "UI size: large",
            "",
            scale == Some(3),
            Some(Intent::SetScale(Some(3))),
        ),
    ]
}

fn window<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let tabs = (context.active_tab, context.tabs.len());
    let cycle = |backwards: bool| neighbour_tab(tabs, backwards).map(Intent::ActivateTab);
    let mut entries = vec![
        entry("Next tab", "Ctrl+Tab", cycle(false)),
        entry("Previous tab", "Ctrl+Shift+Tab", cycle(true)),
        entry(
            "Home",
            "Ctrl+Home",
            context.active_tab.map(|_| Intent::GoHome),
        ),
        separator(),
    ];
    entries.extend(PanelKind::ALL.iter().map(|panel| {
        let torn = context.popped.contains(panel);
        let intent = if torn {
            Intent::Dock(*panel)
        } else {
            Intent::PopOut(*panel)
        };
        check(panel.window_label(), "", torn, Some(intent))
    }));
    entries.push(separator());
    entries.push(entry(
        "New canvas window",
        "Ctrl+Shift+N",
        Some(Intent::NewWindow),
    ));
    entries
}
