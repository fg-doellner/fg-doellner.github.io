#!/bin/bash

jsonfiles=$(ls *.json)

for jsonfile in $jsonfiles
do
  key=$(basename $jsonfile ".json")
  if [ ! -f "$key.pug" ]; then
    cat > $key.pug << EOF
extends /template/newsentry.pug

block newsvariables
  - var newskey = '$key';

//-
  block date
    | [date]
  
  block title
    | [title]

block newscontent
  p.card-text
    | [content]
EOF
  fi
done
