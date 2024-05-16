#[macro_export]
macro_rules! stringify_func_call {
    ($func:ident($($arg:expr),*)) => {{
        let args_str = vec![$(format!("{:?}", $arg)),*].join(", ");
        stringify!($func).to_string() + "(" + &args_str + ")"
    }};
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use crate::metrics;
        ic_cdk::println!($($arg)*);
        ic_utils::logger::log_message(format!($($arg)*));
        ic_utils::monitor::collect_metrics();

        metrics!(set CYCLES, ic_cdk::api::canister_balance() as u128);
    }};
}

#[macro_export]
macro_rules! clone_with_state {
    ($field:ident) => {{
        $crate::STATE.with(|state| state.borrow().$field.clone())
    }};
}

#[macro_export]
macro_rules! update_state {
    ($field:ident, $value:expr) => {{
        $crate::STATE.with(|state| {
            state.borrow_mut().$field = $value;
        })
    }};
}

#[macro_export]
macro_rules! defer {
    ($($code:tt)*) => {
        let _defer = $crate::utils::macros::Defer::new(|| { $($code)* });
    };
}

pub struct Defer<F: FnOnce()> {
    pub f: Option<F>,
}

impl<F: FnOnce()> Defer<F> {
    pub fn new(f: F) -> Defer<F> {
        Defer { f: Some(f) }
    }
}

impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            f();
        }
    }
}

#[macro_export]
macro_rules! retry_until_success {
    ($func:expr) => {{
        const MAX_RETRIES: u32 = 5;
        const DURATION_BETWEEN_ATTEMPTS: std::time::Duration =
            std::time::Duration::from_millis(1000);

        let mut attempts = 0u32;
        let mut result = $func.await;

        while result.is_err()
            && (format!("{:?}", result.as_ref().unwrap_err())
                .contains("Canister http responses were different across replicas")
                || format!("{:?}", result.as_ref().unwrap_err()).contains("Timeout expired"))
            && attempts < MAX_RETRIES
        {
            crate::utils::sleep(DURATION_BETWEEN_ATTEMPTS).await;
            result = $func.await;
            attempts += 1;
        }

        result
    }};
}

#[cfg(test)]
mod macros_tests {

    #[test]
    fn stringify_func_call_test() {
        let func = stringify_func_call!(func(1, 2, 3));
        assert_eq!(func, "func(1, 2, 3)");

        #[derive(Debug)]
        struct Arg3 {
            _field1: i32,
            _field2: String,
        }

        let arg1 = 1;
        let arg2 = "2";
        let arg3 = Arg3 {
            _field1: 3,
            _field2: "3".to_string(),
        };
        let func = stringify_func_call!(another_func(arg1, arg2, arg3, false, 123));

        assert_eq!(
            func,
            "another_func(1, \"2\", Arg3 { _field1: 3, _field2: \"3\" }, false, 123)"
        );
    }
}
