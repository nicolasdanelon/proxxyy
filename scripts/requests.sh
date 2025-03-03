#!/bin/bash

# can this be done in parallel?
# should I be using curl?
# should I do this with rust?

wget -O - http://localhost:6969/people/1/ > /dev/null 
wget -O - http://localhost:6969/people/?page=2 > /dev/null
wget -O - http://localhost:6969/people/?page=3 > /dev/null
