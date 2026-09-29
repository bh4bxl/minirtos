pub trait Service {
    fn run(&mut self) -> !;
}
