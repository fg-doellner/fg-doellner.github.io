#!/bin/bash

bibfiles=$(ls *.bib)

for bibfile in $bibfiles
do
  key=$(basename $bibfile ".bib")
  if [ ! -f "$key.pug" ]; then
    cat > $key.pug << EOF
extends /template/publicationentry.pug

block publicationvariables
  - var bibkey = '$key';

block publicationscontent
  //-
    .row
      .col-12
EOF
  fi
done
