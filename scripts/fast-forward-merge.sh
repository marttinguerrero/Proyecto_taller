#! /usr/bin/bash

pwd 
printf "input executable:\n"
read EXECUTABLE


echo ====================================================================================

printf "COMMAND = init\n\n"
$EXECUTABLE init

echo ====================================================================================

printf "COMMAND = config\n\n"
$EXECUTABLE config --test

echo ====================================================================================

printf "COMMAND = touch common\n"
touch common

echo ====================================================================================

printf "COMMAND = add common\n"
$EXECUTABLE add common

echo ====================================================================================

printf "COMMAND = log\n\n"
$EXECUTABLE log

echo ====================================================================================

printf "COMMAND = branch\n\n"
$EXECUTABLE branch

echo ====================================================================================

printf "COMMAND = commit 'first commit'\n\n"
$EXECUTABLE commit "first commit"

echo ====================================================================================

printf "COMMAND = log\n\n"
$EXECUTABLE log

echo ====================================================================================

printf "COMMAND = branch\n\n"
$EXECUTABLE branch

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = branch new_branch\n\n"
$EXECUTABLE branch new_branch

echo ====================================================================================

printf "COMMAND = branch\n\n"
$EXECUTABLE branch

echo ====================================================================================

printf "COMMAND = checkout new_branch\n\n"
$EXECUTABLE checkout new_branch

echo ====================================================================================

printf "COMMAND = branch\n\n"
$EXECUTABLE branch

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = echo change >> common\n"
echo change >> common

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = add common\n"
$EXECUTABLE add common
$EXECUTABLE add common
$EXECUTABLE add common
$EXECUTABLE add common

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = commit 'from branch'\n\n"
$EXECUTABLE commit "from branch"

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = log\n\n"
$EXECUTABLE log

echo ====================================================================================

printf "COMMAND = checkout master\n\n"
$EXECUTABLE checkout master

echo ====================================================================================

printf "COMMAND = branch\n\n"
$EXECUTABLE branch

echo ====================================================================================

printf "COMMAND = log\n\n"
$EXECUTABLE log

echo ====================================================================================

printf "COMMAND = cat common\n\n"
cat common

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = merge new_branch\n\n"
$EXECUTABLE merge new_branch

echo ====================================================================================

printf "COMMAND = status\n\n"
$EXECUTABLE status

echo ====================================================================================

printf "COMMAND = log \n\n"
$EXECUTABLE log

echo ====================================================================================

printf "COMMAND = branch\n\n"
$EXECUTABLE branch

echo ====================================================================================

rm common
rm -rf .git-rustico
