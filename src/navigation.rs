#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppView {
    Home,
    Systems,
    Catalog { system_id: i64 },
    Details { game_id: i64 },
}

#[derive(Debug, Clone)]
pub struct Navigator {
    history: Vec<AppView>,
}

impl Default for Navigator {
    fn default() -> Self {
        Self {
            history: vec![AppView::Home],
        }
    }
}

impl Navigator {
    pub fn current(&self) -> &AppView {
        self.history
            .last()
            .expect("navigation history is never empty")
    }

    fn push(&mut self, view: AppView) {
        if self.current() != &view {
            self.history.push(view);
        }
    }

    pub fn open_systems(&mut self) {
        self.push(AppView::Systems);
    }
    pub fn open_catalog(&mut self, system_id: i64) {
        self.push(AppView::Catalog { system_id });
    }
    pub fn open_details(&mut self, game_id: i64) {
        self.push(AppView::Details { game_id });
    }

    pub fn home(&mut self) {
        self.history.clear();
        self.history.push(AppView::Home);
    }

    pub fn back(&mut self) {
        if self.history.len() > 1 {
            self.history.pop();
        }
    }
}
