pub trait EmfConverter {
    fn emf_to_svg(&mut self, emf: &[u8]) -> Result<String, String>;
}
