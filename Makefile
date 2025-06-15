test-coverage:
	cargo install cargo-tarpaulin
	cargo tarpaulin --out Html
	open tarpaulin-report.html
