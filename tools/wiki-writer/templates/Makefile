.PHONY: install serve wikify build status lint clean

install:
	sh state/install.sh

serve:
	php -S 127.0.0.1:8080 -t dokuwiki

wikify:
	wiki-writer run --project .

build:
	wiki-writer render --project .
	php dokuwiki/bin/indexer.php -c

status:
	wiki-writer status --project .

lint:
	wiki-writer lint --project .

clean:
	@test ! -L dokuwiki || { echo 'Refusing symlink dokuwiki' >&2; exit 1; }
	rm -rf -- dokuwiki
