# @template/.just/github.just

# Variables.
# Return the repository name.
repo := if which("gh") == [] { '' } else { shell("gh repo view --json name --jq '.name'") }

# Return the repository owner (organization).
owner := if which("gh") == [] { '' } else { shell("gh repo view --json owner --jq '.owner.login'") }

# Return the GitHub user name.
user := if which("gh") == [] { '' } else { shell("gh api user --jq '.login'") }

# GitHub auth token.
github_token := if env('GITHUB_TOKEN', '') != '' { env('GITHUB_TOKEN') } else if which("gh") == [] { '' } else { shell('gh auth token 2>/dev/null || echo ""') }

# CI job name required by the status check ruleset.
status_check := 'Status check'

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
			--persona=auditor . ; \
	else \
		echo "{{ warning('No actions in .github/workflows') }}"; \
	fi

	echo {{ done('') }}

# Update and pin action versions with cooldown
[env("GITHUB_TOKEN", github_token)]
[group('GitHub Actions')]
[no-exit-message]
update_actions:
	echo {{ CYAN }}Update and pin actions in {{ YELLOW }}.github/workflows/*.yaml \
		{{ CYAN }}with a {{ GREEN }}cooldown of 7 days{{ NORMAL }}

	just check_command_pinact

	pinact \
		run \
		--update \
		--verify \
		--min-age 7 \
		--log-level warn \
	|| exit 0

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

	if ! pinact run --check --verify; then \
		echo "{{ error([BOLD + 'pinact', 'detected invalid action versions, run', MAGENTA + 'just update_actions']) }}"; \
		exit 1; \
	fi

	echo {{ done('') }}

# List action secrets for the environment
[group('GitHub Actions')]
env_list_secrets env='production' repo=repo:
	echo {{ CYAN }}List the {{ BOLD }}action secrets{{ NORMAL + CYAN }} for the \
		{{ YELLOW + env + NORMAL + CYAN }} environment in {{ GREEN + owner + '/' + repo + NORMAL }}

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
	echo {{ CYAN }}List {{ BOLD }}all secrets{{ NORMAL + CYAN }} that are \
		{{ YELLOW + 'not environment restricted' + NORMAL + CYAN }} in \
		{{ GREEN + owner + '/' + repo + NORMAL }}

	just check_command_gh

	gh \
		secret \
		list \
		--repo {{ owner + '/' + repo }}

	echo {{ warning('Secrets should be restricted to environments!') }}

	echo {{ done('') }}

# Add an action secret to an environment
[group('GitHub Actions')]
set_env_secret secret env:
	echo {{ CYAN }}Add the {{ BOLD }}action secret \
		{{ NORMAL + YELLOW + uppercase(secret) + CYAN }} to the  {{ BOLD + env + NORMAL + CYAN }} \
		environment in {{ GREEN + owner + '/' + repo + NORMAL }}

	just check_command_gh

	gh \
		secret \
		set {{ uppercase(secret) }} \
		--env {{ env }} \
		--repo {{ owner + '/' + repo }} \
		--app actions

	echo {{ done('') }}

# Create an environment
[arg('env', long='env', help='Environment name')]
[arg('repo', long='repo', value=repo, help='Repository name')]
[group('GitHub Actions')]
create_env env repo=repo:
	echo {{ CYAN }}Create the {{ YELLOW + env + CYAN }} environment in \
		{{ GREEN + owner + '/' + repo + NORMAL }}

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

	echo {{ CYAN }}Set the branch protection on {{ GREEN + main_branch + NORMAL }}
	gh \
		api \
		--method POST \
		repos/{{ owner }}/{{ repo }}/environments/{{ env }}/deployment-branch-policies \
		--field name=main \
		--jq '.total_count'

	echo {{ done([repo, env]) }}

# List the environments
[group('GitHub Actions')]
list_envs repo=repo:
	echo {{ CYAN }}List environments in {{ GREEN + owner }}/{{ BOLD + repo + NORMAL }}

	just check_command_gh

	gh \
		api \
		repos/{owner}/{{ repo }}/environments \
		--jq '.environments[].name'

	echo {{ done(repo) }}

# Check the security policies on a repository
[group('GitHub Actions')]
[no-exit-message]
check_rulesets_repo repo=repo:
	echo {{ CYAN }}Check {{ BOLD }}rulesets{{ NORMAL + CYAN }} in \
	{{ YELLOW + owner }}/{{ BOLD + repo + NORMAL }}

	just check_command_gh
	just check_command_jq

	just check_main_branch_name {{ owner }} {{ repo }}
	just check_branch_protection {{ owner }} {{ repo }}
	just check_rulesets {{ owner }} {{ repo }}

	echo {{ done('') }}

# Check the security policies all organization repositories
[group('GitHub Actions')]
[no-exit-message]
check_rulesets_org owner=owner:
	echo {{ CYAN + UNDERLINE }}Check the {{ BOLD }}organization rulesets \
		{{ NORMAL + UNDERLINE + CYAN }} in {{ YELLOW + BOLD + owner + NORMAL }}

	just check_command_gh
	just check_command_jq

	failed=''; \
	for repo_name in $(gh repo list {{ owner }} --limit 200 --no-archived --json name \
		--jq '.[].name' | sort); do \
		echo "\n{{ YELLOW + BOLD }}$repo_name:{{ NORMAL }}"; \
		just check_main_branch_name {{ owner }} $repo_name; \
		just check_branch_protection {{ owner }} $repo_name; \
		just check_rulesets {{ owner }} "$repo_name" || failed="${failed:+$failed }$repo_name"; \
	done; \
	if [ -n "$failed" ]; then \
		echo "\n{{ error(['Missing rulesets in these repositories:']) }}"; \
		for failed_repo in $failed; do \
			echo "{{ YELLOW }}  $failed_repo"; \
		done; \
	else \
		echo "{{ done(owner) }}"; \
	fi

# Check the rulesets
[group('GitHub Actions')]
[no-exit-message]
[private]
check_rulesets owner=owner repo=repo:
	if ! rules=$(gh api repos/{{ owner }}/{{ repo }}/rules/branches/{{ main_branch }} \
		2>/dev/null); then \
		rules='[]'; \
	fi; \
	missing=''; \
	for rule in \
		required_signatures \
		pull_request \
		required_status_checks \
		deletion \
		non_fast_forward \
		required_linear_history; \
	do \
		source=$(printf '%s' "$rules" | jq -r --arg rule "$rule" \
			'[.[]|select(.type==$rule)|.ruleset_source_type]|unique|join(", ")'); \
		if [ -n "$source" ]; then \
			if [ "$source" = "Organization" ]; then \
				echo "{{ dim([GREEN + '$rule', 'from', GREEN + '$source']) }}"; \
			else \
				echo "{{ dim([GREEN + '$rule', 'from', YELLOW + '$source']) }}"; \
			fi; \
		else \
			echo "{{ dim([NORMAL + RED + BOLD + '$rule', RED + 'is not set']) }}"; \
			missing="$missing $rule"; \
		fi; \
	done; \
	if printf '%s' "$rules" | jq -e 'any(.[]; .type=="required_status_checks")' >/dev/null; then \
		if printf '%s' "$rules" | jq -e --arg check '{{ status_check }}' \
			'any(.[]; .type=="required_status_checks" and \
				any(.parameters.required_status_checks[]?; .context==$check))' >/dev/null; then \
			echo "{{ dim([GREEN + status_check, 'is required']) }}"; \
		else \
			echo "{{ warning([status_check, 'is not a required check']) }}"; \
			missing="$missing status_check"; \
		fi; \
		if printf '%s' "$rules" | jq -e 'any(.[]; .type=="required_status_checks" and \
			.parameters.strict_required_status_checks_policy==true)' >/dev/null; then \
			echo "{{ dim([GREEN + 'Branches must be up to date before merging']) }}"; \
		else \
			echo "{{ warning('Branches can be out of date before merging') }}"; \
			missing="$missing strict_status_checks"; \
		fi; \
	fi; \
	if ! ids=$(gh api repos/{{ owner }}/{{ repo }}/rulesets --jq '.[].id' 2>/dev/null); then \
		ids=''; \
	fi; \
	for id in $ids; do \
		ruleset=$(gh api repos/{{ owner }}/{{ repo }}/rulesets/"$id" 2>/dev/null); \
		name=$(printf '%s' "$ruleset" | jq -r '.name'); \
		enforcement=$(printf '%s' "$ruleset" | jq -r '.enforcement'); \
		if [ "$enforcement" != active ]; then \
			echo "{{ warning(['The ruleset', BOLD + '$name', 'is', BOLD + '$enforcement']) }}"; \
		fi; \
		bypass=$(printf '%s' "$ruleset" | \
			jq -r '[.bypass_actors[]?|select(.bypass_mode=="always")|.actor_type| \
				select(.!="OrganizationAdmin")]|join(", ")'); \
		if [ -n "$bypass" ]; then \
			echo "{{ warning(['The ruleset', BOLD + '$name', 'is bypassed by', BOLD + '$bypass']) }}"; \
		fi; \
	done; \
	if [ -n "$missing" ]; then \
		exit 1; \
	fi

# Check the branch protection status
[group('GitHub Actions')]
[private]
check_branch_protection owner=owner repo=repo:
	if protection=$(gh api repos/{{ owner }}/{{ repo }}/branches/{{ main_branch }}/protection \
		2>/dev/null); then \
		echo "{{ warning('Classic branch protection should be a ruleset') }}"; \
		enabled=$(printf '%s' "$protection" | jq -r \
			'[to_entries[]|select(.value|type=="object")|select(.value.enabled)|.key]|join(" ")'); \
		checks=$(printf '%s' "$protection" | \
			jq -r '(.required_status_checks.contexts//[])|join(", ")'); \
		echo "{{ dim(['Enabled:', BOLD + '${enabled:-none}']) }}"; \
		echo "{{ dim(['Required checks:', BOLD + '${checks:-none}']) }}"; \
	else \
		echo "{{ dim([GREEN + 'Classic branch protection is', GREEN + BOLD + 'not active']) }}"; \
	fi

# Check the name of the main branch
[group('GitHub Actions')]
[private]
check_main_branch_name owner=owner repo=repo:
	main_branch_name=$(gh repo view {{ owner }}/{{ repo }} --json defaultBranchRef \
		--jq '.defaultBranchRef.name' 2>/dev/null); \
	if [ "$main_branch_name" != "{{ main_branch }}" ]; then \
		echo "{{ error(['Invalid main branch name in', BOLD + owner + '/' + repo]) }}"; \
		echo "{{ warning(['Rename the main branch', RED + '$main_branch_name', 'to', GREEN + main_branch]) }}"; \
	else \
		echo "{{ dim([GREEN + 'Main branch name is', GREEN + BOLD + main_branch]) }}"; \
	fi

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
	echo "{{ dim(shell('pinact --version 2>/dev/null')) }}"

# Check if `zizmor` is available
[group('GitHub Actions')]
[private]
check_command_zizmor:
	just check_command zizmor 'just install_cargo_tools' 'just install_cargo_tools'
