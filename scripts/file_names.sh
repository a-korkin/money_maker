#!/bin/bash

source .env
    
rename 's/([0-9]{4}-[0-9]{2}-[0-9]{2}_)([0-9]{1})(\.csv)/${1}0${2}${3}/' \
    $DATA_DIR/candles/**/*.csv
