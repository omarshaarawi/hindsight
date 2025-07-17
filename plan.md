# Hindsight Development Plan

A minimal shell history search tool with fuzzy finding via ctrl-r.

## Overview
- Fork of zhistory focused only on search functionality
- No recording, stats, or sync features
- Zsh only support
- Reads from existing shell history database

## Phase 1: Project Setup
- [x] Initialize jj repository  
- [ ] Create rust project structure
- [ ] Initial commit

## Phase 2: Core Search
- [ ] Basic CLI with clap
- [ ] SQLite database reader
- [ ] Fuzzy search with skim
- [ ] Connect search to database

## Phase 3: Shell Integration  
- [ ] Zsh ctrl-r binding
- [ ] Handle search modes (global/session/cwd)
- [ ] Return selected command

## Phase 4: Polish
- [ ] Config file support
- [ ] Error handling
- [ ] Install script

## Technical Details
- Use existing zhistory database format for compatibility
- Three search modes: global, session, cwd
- Config in ~/.config/hindsight/config.toml
- Simple install script to add to .zshrc