//! Macros de registro declarativo de handlers.

/// Registra vários [`crate::CommandHandler`]s no registry.
///
/// Uso:
///
/// ```rust,ignore
/// use gerax_cqrs::{register_commands, HandlerRegistry};
///
/// let mut registry = HandlerRegistry::new();
/// register_commands!(registry, my_command_handler);
/// ```
///
/// Expande para:
///
/// ```rust,ignore
/// registry.register_command(my_command_handler)?;
/// ```
#[macro_export]
macro_rules! register_commands {
    ($registry:ident, $($handler:expr),+ $(,)?) => {
        $(
            $registry.register_command($handler)?;
        )+
    };
}

/// Registra vários [`crate::QueryHandler`]s no registry.
///
/// Uso:
///
/// ```rust,ignore
/// use gerax_cqrs::{register_queries, HandlerRegistry};
///
/// let mut registry = HandlerRegistry::new();
/// register_queries!(registry, my_query_handler);
/// ```
///
/// Expande para:
///
/// ```rust,ignore
/// registry.register_query(my_query_handler)?;
/// ```
#[macro_export]
macro_rules! register_queries {
    ($registry:ident, $($handler:expr),+ $(,)?) => {
        $(
            $registry.register_query($handler)?;
        )+
    };
}
