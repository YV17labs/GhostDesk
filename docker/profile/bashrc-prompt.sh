# The original __bash_prompt embeds multi-line backtick substitutions as
# literal strings inside PS1, which bash re-parses at every prompt display
# and fails with "command substitution: line 12: syntax error: unexpected end
# of file". Fix: use PROMPT_COMMAND so git runs in a normal function (with
# $()) rather than as a string re-evaluated inside PS1 each time.
unset PROMPT_DIRTRIM
_ghostdesk_prompt() {
    local _exit=$?
    local _branch
    # One fork for the common cases (on a branch, or outside a git repo).
    # Falls back to a second call only in detached-HEAD state.
    _branch=$(git --no-optional-locks rev-parse --abbrev-ref HEAD 2>/dev/null) || true
    if [ "$_branch" = HEAD ]; then
        _branch=$(git --no-optional-locks rev-parse --short HEAD 2>/dev/null) || true
    fi
    local _CYN='\[\033[0;36m\]'
    local _RED='\[\033[1;31m\]'
    local _GRN='\[\033[0;32m\]'
    local _BLU='\[\033[1;34m\]'
    local _RST='\[\033[0m\]'
    local _arrow; [ "$_exit" -eq 0 ] && _arrow="${_RST}➜" || _arrow="${_RED}➜"
    local _user; [ -n "${GITHUB_USER:-}" ] && _user="${_GRN}@${GITHUB_USER} " || _user="${_GRN}\u "
    local _git; [ -n "${_branch}" ] && _git="${_CYN}(${_RED}${_branch}${_CYN}) " || _git=""
    PS1="${_user}${_arrow} ${_BLU}\w ${_git}${_RST}\$ "
}
PROMPT_COMMAND=_ghostdesk_prompt
