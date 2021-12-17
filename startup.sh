#!/bin/bash

# This script will clone the dev env, then launch it. 

CUR_DIR=${PWD}
DEV_ENV=.dev_env

# Clone dev-env if it doesn't exist
[ ! -d ${DEV_ENV} ] && git clone https://github.com/ericrobolson/dev-env.git ${DEV_ENV}

# Update dev-env
cd ${DEV_ENV}
git checkout -- . 
git pull

# Run nix
nix-shell
