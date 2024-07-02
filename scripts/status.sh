#! /usr/bin/bash

# Creates 3 files and operates on them so that
# the status command shows one on each category:
# untracked, staged, not staged

printf "input executable:\n"
read EXECUTABLE
# EXECUTABLE=git-rustico

echo ====================================================================================

printf "COMMAND = init\n\n"
$EXECUTABLE init

echo ====================================================================================

touch untracked
printf "COMMAND = touch untracked\n"

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = touch staged modified\n"
touch staged
touch modified

echo ====================================================================================

printf "COMMAND = add staged modified\n"
$EXECUTABLE add staged modified

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = echo change > modified\n"
echo "change" > modified

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

rm untracked
rm staged
rm modified
rm -rf .git-rustico
