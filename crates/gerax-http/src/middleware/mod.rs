
pub trait Middleware: Send + Sync + 'static {
    fn name(&self) -> &str;
}
