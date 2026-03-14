test-build-feature:
	./bin/build-feature test-feature docs/ "This is a test prompt"

test-gen-doc:
	./bin/gen-doc SonataOverview Give me an overview of a sonata musical form.

test-clean-room:
	./bin/clean-room test-feature ./bin ./specs ./impl "Reimplement the helper utilities"

test-append-prompt:
	bash tests/test-append-prompt.sh