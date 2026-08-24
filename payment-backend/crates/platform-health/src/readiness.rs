use std::future::Future;

pub struct ReadinessCheck {
    pub name: &'static str,
    pub check_fn: Box<dyn Fn() -> Box<dyn Future<Output = bool> + Unpin + Send> + Send + Sync>,
}

impl ReadinessCheck {
    pub fn new(name: &'static str, f: impl Fn() -> Box<dyn Future<Output = bool> + Unpin + Send> + Send + Sync + 'static) -> Self {
        Self { name, check_fn: Box::new(f) }
    }
}

pub struct Readiness {
    pub checks: Vec<ReadinessCheck>,
}

impl Default for Readiness {
    fn default() -> Self {
        Self::new()
    }
}

impl Readiness {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn add_check(&mut self, check: ReadinessCheck) {
        self.checks.push(check);
    }
}
