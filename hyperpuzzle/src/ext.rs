pub trait OptionStoreOnce<T> {
    fn store(&mut self, value: T) -> Result<(), ()>;
}
impl<T> OptionStoreOnce<T> for Option<T> {
    fn store(&mut self, value: T) -> Result<(), ()> {
        match self {
            Some(v) => {
                *v = value;
                Ok(())
            }
            None => Err(()),
        }
    }
}
