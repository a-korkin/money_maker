#!/bin/bash

source .env
# DIR_CANDLES=$DATA_DIR/candles/OZON
DIR_CANDLES=data/candles/OZON

find $DIR_CANDLES -regextype posix-extended \
    -regex "$DIR_CANDLES/[[:digit:]]{4}-[[:digit:]]{2}-[[:digit:]]{2}_([[:digit:]]{1})\.csv"
    
