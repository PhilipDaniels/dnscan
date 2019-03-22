pub trait AsStr {
    fn as_str(&self) -> &'static str;
}

impl AsStr for bool {
    fn as_str(&self) -> &'static str {
        if *self { "true" } else { "false" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_bool() {
        assert_eq!(true.as_str(), "true");
    }
}