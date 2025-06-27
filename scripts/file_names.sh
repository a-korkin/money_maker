#!/bin/bash

source .env
DIR_CANDLES=$DATA_DIR/candles
# DIR_CANDLES=data/candles
    
rename 's/([0-9]{4}-[0-9]{2}-[0-9]{2}_)([0-9]{1})(\.csv)/${1}0${2}${3}/' \
    $DIR_CANDLES/**/*.csv
