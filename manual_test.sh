set -e
# Setup
export SHADOWBOX_FILE=.shadowbox_test
export GITIGNORE_FILE=.gitignore_test
touch test_file.txt
rm -f $SHADOWBOX_FILE $GITIGNORE_FILE

# 1. Track a file
cargo run -- track test_file.txt
echo "--- After tracking test_file.txt ---"
cat $SHADOWBOX_FILE

# 2. Track '.'
cargo run -- track .
echo "--- After tracking '.' ---"
cat $SHADOWBOX_FILE

# 3. Untrack '.'
cargo run -- untrack .
echo "--- After untracking '.' ---"
cat $SHADOWBOX_FILE

# 4. Untrack test_file.txt
cargo run -- untrack test_file.txt
echo "--- After untracking test_file.txt ---"
cat $SHADOWBOX_FILE

# Cleanup
rm test_file.txt $SHADOWBOX_FILE $GITIGNORE_FILE
