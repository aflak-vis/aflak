/// Try to convert an integer into a usize.
/// Return a `Result<usize, IOErr>`.
macro_rules! try_into_unsigned {
    ($value: ident) => {
        if $value >= 0 {
            Ok($value as usize)
        } else {
            Err($crate::IOErr::UnexpectedInput(format!(
                "'{}' must be positive, but got {}",
                stringify!($value),
                $value
            )))
        }
    };
}

/// Check that a custom condition is fulfilled.
macro_rules! precheck {
    ($start: ident $op :tt $end: ident) => {
        if $start $op $end {
            Ok(())
        } else {
            Err($crate::IOErr::UnexpectedInput(format!(
                "Expected {} {} {}, but got {} {} {}",
                stringify!($start),
                stringify!($op),
                stringify!($end),
                $start,
                stringify!($op),
                $end,
            )))
        }
    };
    ($cond: expr, $($arg:tt)*) => {
        if $cond {
            Ok(())
        } else {
            Err($crate::IOErr::UnexpectedInput(format!($($arg)*)))
        }
    };
}

/// Check that a WcsArray has more than 0 dimensions.
/// If so, return the number of frames along the first dimension.
macro_rules! has_gt_0_dim {
    ($wcs_array: ident) => {
        if let Some(frame_cnt) = $wcs_array.scalar().dim().as_array_view().first() {
            Ok(*frame_cnt)
        } else {
            Err($crate::IOErr::UnexpectedInput(format!(
                "'{}' is a 0-dimensional image, cannot slice",
                stringify!($wcs_array)
            )))
        }
    };
}

/// Check that a WcsArray is sliceable from indices 'start' to 'end'.
macro_rules! is_sliceable {
    ($wcs_array: ident, $frame_idx: ident) => {
        has_gt_0_dim!($wcs_array).and_then(|frame_cnt| {
            precheck!(
                $frame_idx <= frame_cnt,
                "'{}' greater or equal to the input image '{}''s frame count (expected {} <= {})",
                stringify!($frame_idx), stringify!($wcs_array), $frame_idx, frame_cnt
            )
        })
    };
    ($wcs_array: ident, $start: ident, $end: ident) => {
        has_gt_0_dim!($wcs_array).and_then(|frame_cnt| {
            precheck!($start < $end).and_then(|_| {
                precheck!(
                    $end <= frame_cnt,
                    "'{}' greater or equal to the input image '{}''s frame count (expected {} <= {})",
                    stringify!($end), stringify!($wcs_array), $end, frame_cnt
                )
            })
        })
    };
}
