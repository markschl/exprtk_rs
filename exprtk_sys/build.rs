
extern crate gcc;


fn main() {
	let mut c = gcc::Build::new();

	c.file("cpp/wrapper.cpp")
		.cpp(true);

	 if cfg!(target_os = "windows") {
		c.flag("-bigobj");
	}

	if cfg!(feature = "debug") {
		c.define("exprtk_enable_debugging", Some("1"));
	}
	if cfg!(not(feature = "comments")) {
		c.define("exprtk_disable_comments", Some("1"));
	}
	if cfg!(not(feature = "break_continue")) {
		c.define("exprtk_disable_break_continue", Some("1"));
	}
	if cfg!(not(feature = "sc_andor")) {
		c.define("exprtk_disable_sc_andor", Some("1"));
	}
	if cfg!(not(feature = "return_statement")) {
		c.define("exprtk_disable_return_statement", Some("1"));
	}
	if cfg!(not(feature = "enhanced_features")) {
		c.define("exprtk_disable_enhanced_features", Some("1"));
	}
	if cfg!(not(feature = "superscalar_unroll")) {
		c.define("exprtk_disable_superscalar_unroll", Some("1"));
	}
	if cfg!(not(feature = "rtl_io_file")) {
		c.define("exprtk_disable_rtl_io_file", Some("1"));
	}
	if cfg!(not(feature = "rtl_vecops")) {
		c.define("exprtk_disable_rtl_vecops", Some("1"));
	}
	if cfg!(not(feature = "caseinsensitivity")) {
		c.define("exprtk_disable_caseinsensitivity", Some("1"));
	}

	c.compile("libexprtk.a");
}
