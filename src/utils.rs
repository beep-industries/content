use mockall::automock;

pub struct RealTime {}

#[automock]
pub trait Time {
    fn now(&self) -> u64;
}
impl Time for RealTime {
    fn now(&self) -> u64 {
        chrono::Utc::now()
            .timestamp()
            .try_into()
            .unwrap_or_default()
    }
}

pub fn get_time() -> RealTime {
    RealTime {}
}

#[cfg(test)]
pub mod tests {
    use crate::utils::MockTime;

    pub fn get_time() -> MockTime {
        let mut mock = MockTime::new();
        mock.expect_now().returning(|| 100);
        mock
    }
}
