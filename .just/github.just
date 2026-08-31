# .just/github.just

# Variables.
# Return the repository name.
repo := if which("gh") == [] { '' } else { shell("gh repo view --json name --jq '.name'") }
# Return the repository owner (organization).
owner := if which("gh") == [] { '' } else { shell("gh repo view --json owner --jq '.owner.login'") }
# Return the GitHub user name.
user := if which("gh") == [] { '' } else { shell("gh api user --jq '.login'") }
# GitHub auth token.
github_token := if which("gh") == [] { '' } else { shell('gh auth token') }

# Audit actions
[group('GitHub Actions')]
[no-exit-message]
audit_actions:
	echo {{ CYAN }}Audit actions in {{ GREEN }}.github/workflows/*.yaml{{ NORMAL }}
	just check_command_zizmor
	if [ -d ".github/workflows" ]; then \
		zizmor \
			--no-progress \
			{{ if github_token != '' { '--gh-token ' + github_token } }} \
			--persona=auditor . \
	;else \
		echo "{{ warning('No actions in .github/workflows') }}" \
	;fi
	echo {{ done('') }}

# Update and pin action versions with cooldown
[env("GITHUB_TOKEN", github_token)]
[group('GitHub Actions')]
[no-exit-message]
update_actions:
	echo {{ CYAN }}Update and pin actions in {{ YELLOW }}.github/workflows/*.yaml \
		{{ CYAN }} with a {{ GREEN }}cooldown of 7 days{{ NORMAL }}
	just check_command_pinact
	pinact \
		run \
		--update \
		--verify \
		--min-age 7 \
		--log-level warn || \
	exit 0
	echo {{ done('') }}

# Update and pin action versions
[env("GITHUB_TOKEN", github_token)]
[group('GitHub Actions')]
[private]
update_actions_latest:
	echo {{ CYAN }}Update and pin actions in {{ YELLOW }}.github/workflows/*.yaml \
		{{ CYAN }} with {{ RED }}no cooldown{{ NORMAL }}
	just check_command_pinact
	pinact \
		run \
		--update \
		--verify \
		--log-level warn
	echo {{ done('') }}

# Check action versions
[env("GITHUB_TOKEN", github_token)]
[group('GitHub Actions')]
check_actions:
	echo {{ CYAN }}Check actions in {{ GREEN }}.github/workflows/*.yaml{{ NORMAL }}
	just check_command_pinact
	pinact \
		run \
		--check \
		--verify
	echo {{ done('') }}

# List action secrets for the environment
[group('GitHub Actions')]
env_list_secrets env='production' repo=repo:
	echo {{ CYAN }}List the {{ BOLD }}action secrets{{ NORMAL + CYAN }} for the {{ YELLOW + env + NORMAL + CYAN }} environment in {{ GREEN + owner + '/' + repo + NORMAL }}
	just check_command_gh
	gh \
		secret \
		list \
		--env {{ env }} \
		--repo {{ owner + '/' + repo }} \
		--app actions
	echo {{ done('') }}

# List all secrets not environment restricted
[group('GitHub Actions')]
env_list_secrets_all repo=repo:
	echo {{ CYAN }}List {{ BOLD }}all secrets{{ NORMAL + CYAN }} that are {{ YELLOW + 'not environment restricted' + NORMAL + CYAN }} in {{ GREEN + owner + '/' + repo + NORMAL }}
	just check_command_gh
	gh \
		secret \
		list \
		--repo {{ owner + '/' + repo }}
	echo {{ YELLOW }}Secrets should be restricted to environments!{{ NORMAL }}
	echo {{ done('') }}

# Add to an environment an action `secret`
[group('GitHub Actions')]
env_set_secret env secret:
	echo {{ CYAN }}Add the action secret {{ BOLD + YELLOW + uppercase(secret) + NORMAL + CYAN }} to the {{ YELLOW + env + CYAN }} environment in {{ GREEN + owner + '/' + repo + NORMAL }}
	just check_command_gh
	gh \
		secret \
		set {{ uppercase(secret) }} \
		--env {{ env }} \
		--repo {{ owner + '/' + repo }} \
		--app actions
	echo {{ done('') }}

# Create an environment
[group('GitHub Actions')]
create_env repo=repo env='production':
	echo {{ CYAN }}Create the {{ YELLOW + env + CYAN }} environment in {{ GREEN + owner + '/' + repo + NORMAL }}
	just check_command_gh
	gh \
		api \
		--method PUT \
		repos/{{ owner }}/{{ repo }}/environments/{{ env }} \
		--jq '.created_at'

	echo {{ CYAN }}Set a custom branch policy{{ NORMAL }}
	gh \
		api \
		--method PUT repos/{{ owner }}/{{ repo }}/environments/{{ env }} \
		--field deployment_branch_policy[protected_branches]=false \
		--field deployment_branch_policy[custom_branch_policies]=true \
		--jq '.deployment_branch_policy.custom_branch_policies'

	echo {{ CYAN }}Set the branch protection to the {{ GREEN + main_branch + CYAN }} branch{{ NORMAL }}
	gh \
		api \
		--method POST \
		repos/{{ owner }}/{{ repo }}/environments/{{ env }}/deployment-branch-policies \
		--field name=main \
		--jq '.total_count'

	echo {{ done('') }}

# List the environments
[group('GitHub Actions')]
list_envs repo=repo:
	echo {{ CYAN }}List environments in {{ GREEN + owner }}/{{ BOLD + repo + NORMAL }}
	just check_command_gh
	gh api repos/{{ owner }}/{{ repo }}/environments --jq '.environments[].name'

## Check commands.

# Check if `gh` is available
[group('GitHub Actions')]
[private]
check_command_gh:
	just check_command gh 'brew install gh' 'winget install --id GitHub.cli'

# Check if `jq` is available
[group('GitHub Actions')]
[private]
check_command_jq:
	just check_command jq 'brew install jq' 'winget install --id jqlang.jq'

# Check if `pinact` is available
[group('GitHub Actions')]
[private]
check_command_pinact:
	just check_command pinact 'brew install pinact' 'winget install --id suzuki-shunsuke.pinact'

# Check if `zizmor` is available
[group('GitHub Actions')]
[private]
check_command_zizmor:
	just check_command zizmor 'just install_cargo_tools' 'just install_cargo_tools'
