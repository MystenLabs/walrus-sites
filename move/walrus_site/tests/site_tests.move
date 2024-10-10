#[test_only]
module walrus_site::site_tests {
    use walrus_site::site::{
        ERangeStartGreaterThanRangeEnd
    };
    #[test]
    fun test_new_range_no_bounds_defined() {
        walrus_site::site::new_range(
            option::none(),
            option::none()
        );
    }
    #[test]
    fun test_new_range_both_bounds_defined() {
        walrus_site::site::new_range(
            option::some(0),
            option::some(1)
        );
    }
    #[test]
    fun test_new_range_only_upper_bound_defined() {
        walrus_site::site::new_range(
            option::none(),
            option::some(1024)
        );
    }
    #[test]
    fun test_new_range_only_lower_bound_defined() {
        walrus_site::site::new_range(
            option::some(1024),
            option::none()
        );
    }
    #[test]
    fun test_new_range_lower_bound_can_be_zero() {
        walrus_site::site::new_range(
            option::some(0),
            option::none()
        );
    }
    #[test]
    #[expected_failure(abort_code = ERangeStartGreaterThanRangeEnd)]
    fun test_new_range_upper_cannot_be_less_than_lower_bound() {
        walrus_site::site::new_range(
            option::some(2),
            option::some(1)
        );
    }
}
