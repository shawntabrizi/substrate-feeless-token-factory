#!/bin/bash
for i in $(echo ./part-*)
do
   echo -e "\nInstall and upgrade $i\n";
   cd $i;
   yarn;
   yarn upgrade --latest;
done
