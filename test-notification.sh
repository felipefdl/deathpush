#!/bin/bash
echo "Sleeping 5 seconds..."
sleep 5
printf '\e]9;Task complete\a'
echo "OSC 9 notification sent!"
