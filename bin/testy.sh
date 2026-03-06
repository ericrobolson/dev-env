say_hi() {
    local STDIN_PROMPT=$(cat) 

    echo "Hello, $STDIN_PROMPT"
}

echo "bob" | say_hi