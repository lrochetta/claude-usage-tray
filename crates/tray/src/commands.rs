//! Commands the user can trigger via the tray menu.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    RefreshNow,
    OpenStats,
    OpenClaudeSettings,
    ToggleAutostart,
    Quit,
}

pub struct MenuItemIds {
    pub refresh: String,
    pub stats: String,
    pub claude_settings: String,
    pub autostart: String,
    pub quit: String,
}

impl MenuItemIds {
    pub fn resolve(&self, id: &str) -> Option<Command> {
        if id == self.refresh {
            Some(Command::RefreshNow)
        } else if id == self.stats {
            Some(Command::OpenStats)
        } else if id == self.claude_settings {
            Some(Command::OpenClaudeSettings)
        } else if id == self.autostart {
            Some(Command::ToggleAutostart)
        } else if id == self.quit {
            Some(Command::Quit)
        } else {
            None
        }
    }
}
