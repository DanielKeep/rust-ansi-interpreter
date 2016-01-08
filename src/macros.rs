macro_rules! marker_error {
    (@as_expr $e:expr) => {$e};

    (
        $(#[$attrs:meta])*
        pub struct $name:ident
        impl {
            desc {$($desc:tt)*}
        }
    ) => {
        $(#[$attrs])*
        pub struct $name;

        impl ::std::fmt::Display for $name {
            fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                self.description().fmt(fmt)
            }
        }

        impl ::std::error::Error for $name {
            fn description(&self) -> &str {
                marker_error!(@as_expr {$($desc)*})
            }
        }
    };
}

macro_rules! perror {
    ($($args:expr),* $(,)*) => {
        {
            use ::std::io::Write;
            write!(::std::io::stderr(), $($args),*).unwrap();
        }
    };
}

macro_rules! rethrow {
    ($e:expr) => {
        match $e {
            ::std::result::Result::Ok(v) => ::std::result::Result::Ok(v),
            ::std::result::Result::Err(err) => {
                let err = ::std::convert::From::from(err);
                ::std::result::Result::Err(err)
            }
        }
    };
}

macro_rules! throw {
    ($e:expr) => {
        return ::std::result::Result::Err(::std::convert::From::from($e))
    };
}
