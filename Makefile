install-skills:
	@if [ -z "$(path)" ]; then echo "Usage: make install-skills path=<destination>"; exit 1; fi
	cp -r skills/ $(path)/skills

install-workflows:
	@if [ -z "$(path)" ]; then echo "Usage: make install-workflows path=<destination>"; exit 1; fi
	cp -r workflows/ $(path)/workflows
