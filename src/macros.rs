#[macro_export]
macro_rules! handle_error {
    ($result:ident, $custom_error:expr) => {{
        if let Err(e) = $result {
            let error = format!("{}: \nError Data: {}", $custom_error, e.to_string());
            return Err(error);
        }
        $result.unwrap()
    }};

    ($result:expr, $custom_error:expr) => {{
        let result = $result;
        if let Err(e) = result {
            let error = format!("{}: \nError Data: {}", $custom_error, e.to_string());
            return Err(error);
        }
        result.unwrap()
    }};
}

#[macro_export]
macro_rules! handle_option {
    ($option:expr,$custom_error:expr) => {{
        if let None = $option {
            return Err($custom_error.to_string());
        }
        $option.unwrap()
    }};
    ($option:ident,$custom_error:expr) => {{
        if let None = $option {
            return Err($custom_error.to_string());
        }
        $option.unwrap()
    }};
}
