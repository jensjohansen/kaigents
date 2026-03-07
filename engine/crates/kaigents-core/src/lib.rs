#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunId(pub String);

impl RunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_round_trips() {
        let run_id = RunId::new("run-001");
        assert_eq!(run_id.as_str(), "run-001");
    }
}
