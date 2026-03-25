test-build-feature:
	./bin/build-feature test-feature docs/ "This is a test prompt"

test-gen-doc:
	./bin/gen-doc SonataOverview Give me an overview of a sonata musical form.

test-clean-room:
	./bin/clean-room test-feature ./bin ./specs ./impl "Reimplement the helper utilities"

test-append-prompt:
	bash tests/test-append-prompt.sh

install-skills:
	@if [ -z "$(path)" ]; then echo "Usage: make install-skills path=<destination>"; exit 1; fi
	cp -r skills/ $(path)/skills

finalize:
	claude "Update the README.md to reflect the new features and changes since the last commit"

ensure-consistency:
	claude "Ensure that all skills that are related to each other (f-build-epics & f-plan) are consistent with each other. Ensure that the skills are not duplicated and that the skills are not missing any information."