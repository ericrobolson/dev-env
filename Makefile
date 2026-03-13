test-build-feature:
	./bin/build-feature test-feature docs/ "This is a test prompt"

test-gen-doc:
	./bin/gen-doc SonataOverview Give me an overview of a sonata musical form.

test-clean-room:
	./bin/clean-room test-feature ./bin ./specs ./impl "Reimplement the helper utilities"

test-research-feature:
	./bin/research-feature TestFeature docs "Build a simple key-value store"