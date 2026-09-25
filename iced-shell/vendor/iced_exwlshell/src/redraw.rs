use iced_core::window::Id;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    None,
    Window(Id),
    Windows(Vec<Id>),
    All,
}

pub(crate) enum Targets {
    None,
    Window(Id),
    All,
    Windows(Vec<Id>),
}

type RedrawScopeFn<Message> = dyn Fn(&Message) -> Scope;

pub(crate) struct Policy<Message> {
    scope: Option<Box<RedrawScopeFn<Message>>>,
}

impl<Message> Policy<Message> {
    pub(crate) fn new(scope: impl Fn(&Message) -> Scope + 'static) -> Self {
        Self {
            scope: Some(Box::new(scope)),
        }
    }

    #[cfg(any(all(feature = "debug", not(target_arch = "wasm32")), test))]
    pub(crate) fn for_wrapped_messages<Wrapped>(
        self,
        inner_message: impl for<'a> Fn(&'a Wrapped) -> Option<&'a Message> + 'static,
    ) -> Policy<Wrapped>
    where
        Message: 'static,
    {
        let Some(scope) = self.scope else {
            return Policy::default();
        };

        Policy::new(move |message| inner_message(message).map_or(Scope::All, &scope))
    }

    pub(crate) fn targets(&self, messages: &[Message]) -> Targets {
        let Some(scope) = self.scope.as_ref() else {
            return Targets::All;
        };

        let mut windows = Vec::new();
        for message in messages {
            match scope(message) {
                Scope::All => return Targets::All,
                Scope::None => {},
                Scope::Window(id) => { if !windows.contains(&id) { windows.push(id); } },
                Scope::Windows(ids) => for id in ids { if !windows.contains(&id) { windows.push(id); } },
            }
        }
        match windows.len() {
            0 => Targets::None,
            1 => Targets::Window(windows[0]),
            _ => Targets::Windows(windows),
        }
    }
}

impl<Message> Default for Policy<Message> {
    fn default() -> Self {
        Self { scope: None }
    }
}

impl<Message> Debug for Policy<Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Policy").finish_non_exhaustive()
    }
}
