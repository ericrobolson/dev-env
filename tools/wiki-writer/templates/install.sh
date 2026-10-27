#!/bin/sh
set -eu

url='https://github.com/dokuwiki/dokuwiki/releases/download/release-2026-07-14c/dokuwiki-2026-07-14c.tgz'
digest='7e9b4dcb5ecb91a57f71b9b77dcec7bcf929959881464796611349ed1d183176'
project='@@PROJECT@@'
dest='dokuwiki'

if [ -L "$dest" ]; then
  echo 'Refusing symlink dokuwiki' >&2
  exit 1
fi
if [ -f "$dest/doku.php" ]; then
  wiki-writer doctor
  exit 0
fi
if [ -e "$dest" ]; then
  test -d "$dest" || { echo 'Refusing non-directory dokuwiki' >&2; exit 1; }
  extra=$(find "$dest" -mindepth 1 ! -path "$dest/data" ! -path "$dest/data/pages" ! -path "$dest/data/pages/*" -print -quit)
  links=$(find "$dest" -type l -print -quit)
  if [ -n "$extra" ] || [ -n "$links" ]; then
    echo 'Refusing incomplete nonempty dokuwiki' >&2
    exit 1
  fi
fi

stage=$(mktemp -d .dokuwiki-stage.XXXXXX)
trap 'rm -rf -- "$stage"' EXIT HUP INT TERM
curl -fL --retry 2 "$url" -o "$stage/archive.tgz"
printf '%s  %s\n' "$digest" "$stage/archive.tgz" | shasum -a 256 -c -
mkdir "$stage/wiki"
tar -xzf "$stage/archive.tgz" --strip-components=1 -C "$stage/wiki"
test -f "$stage/wiki/doku.php" || { echo 'DokuWiki archive missing doku.php' >&2; exit 1; }
cat > "$stage/wiki/conf/local.php" <<'PHP'
<?php
$conf['title'] = '@@PROJECT@@';
$conf['userewrite'] = 0;
$conf['useacl'] = 1;
$conf['authtype'] = 'authplain';
$conf['disableactions'] = 'register';
PHP
cat > "$stage/wiki/conf/acl.auth.php" <<'ACL'
# acl.auth.php
# <?php exit() ?>
* @ALL 1
ACL
cat > "$stage/wiki/conf/users.auth.php" <<'USERS'
# users.auth.php
# <?php exit()?>
# Don't modify the lines above
USERS
rm -f -- "$stage/wiki/install.php"
if [ -d "$dest/data/pages" ]; then
  mkdir -p "$stage/wiki/data/pages"
  cp -R -- "$dest/data/pages/." "$stage/wiki/data/pages/"
fi
if [ -d "$dest" ]; then mv -- "$dest" "$stage/old"; fi
if ! mv -- "$stage/wiki" "$dest"; then
  if [ -d "$stage/old" ]; then mv -- "$stage/old" "$dest"; fi
  exit 1
fi
wiki-writer doctor
