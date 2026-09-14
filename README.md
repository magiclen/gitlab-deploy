Gitlab Deploy
====================

[![CI](https://github.com/magiclen/gitlab-deploy/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/gitlab-deploy/actions/workflows/ci.yml)

GitLab Deploy is used for deploying software projects to multiple hosts during different phases. This program should be run on Linux.

## Setup

Run GitLab Deploy on a Linux deployment client. Install the commands required by the selected
workflow, including `ssh`, `scp`, `wget`, `tar`, `bash`, and `zstd`. Backend image deployments also
need Docker on the deployment client. The remote hosts need the commands used by their deployment
scripts, including Docker for backend deployments.

Frontend control hosts also need Bash to check both commands in the archive extraction pipeline.
Frontend publication prepares the new files in a temporary directory next to `html` before switching
directories. The previous directory is restored if the switch fails. The `html` path remains a real
directory; the switch can still leave a short interval when it is absent. If a restore fails, the
error message gives the location of the backup so that it can be recovered manually.

Commands that fetch an archive from GitLab require `--gitlab-api-url-prefix` and
`--gitlab-api-token`. The token needs permission to read the project. Control commands do not fetch
from GitLab, so `frontend-control`, `backend-control`, and `simple-control` do not require these
options.

Configure non-interactive SSH access from the deployment client to every target host. The tool uses
the remote user's home directory and writes deployments under `~/projects`. Frontend commands use
`~/services/www` for published static files.

`simple-control` runs the command written after `--` on every host of the phase. Each argument is
quoted before it is sent, so the remote program receives exactly the arguments given here and shell
features such as `$VAR`, `*`, and `&&` are not expanded remotely. Run them through a shell
explicitly when they are needed, for example `-- bash -c 'systemctl reload nginx && echo done'`.

`--inject-project-directory` inserts the project directory right after the program name, and treats
a leading bare `sudo` as part of the invocation rather than as the program. It does not understand
`sudo` options or an absolute path such as `/usr/bin/sudo`, so write the command as
`-- sudo <program> [arguments]` when the directory has to be injected.

### Phases

Create a phase file at `$HOME/phases/<phase>`. Each non-comment line starts with a GitLab project
ID followed by one or more SSH targets in `user@host` or `user@host:port` form.

```
# $HOME/phases/test
123 deploy@web-01.example.com deploy@web-02.example.com:2222
456 .
```

The `.` target copies the host list from the immediately preceding project line. Project IDs must
be unique within a phase file. Empty lines and text after `#` are ignored.

For a deployment of project `website` with ID `123`, reference `pre-release`, and commit SHA
`0b14cd4f`, the project directory is:

```
~/projects/website-123/pre-release-0b14cd4f
```

### Project Files

Frontend deployment projects need `deploy/build.sh`, `deploy/public-name.txt`, and a build output
named `deploy/<public-name>.tar.zst`. Backend deployment projects need `deploy/build.sh`,
`deploy/develop-up.sh`, `deploy/develop-down.sh`, `deploy/image-name.txt`, and
`deploy/docker-compose.yml` or `deploy/docker-compose.<build-target>.yml`. The Docker Compose file
must contain an untagged `image: <image-name>` entry for the configured image.

### Security

The TLS certificate of the GitLab server is verified. Pass `--no-check-certificate`, or set the
`GITLAB_NO_CHECK_CERTIFICATE` environment variable, to skip the verification when the server uses a
certificate that the deployment client cannot verify, such as a self-signed one. The API token can
then be read by a man in the middle, so prefer installing the certificate authority on the
deployment client instead.

SSH and SCP connect with `StrictHostKeyChecking=accept-new`, which trusts a host that is seen for
the first time and records its key, but refuses to connect once the key of a known host has changed.
Add the keys of the target hosts to `known_hosts` in advance when even the first connection has to
be verified.

Use dedicated deployment credentials with the minimum permissions needed.

## Help

```
EXAMPLES:
gitlab-deploy frontend-develop --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --build-target develop
gitlab-deploy frontend-deploy  --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --build-target test
gitlab-deploy frontend-control --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test
gitlab-deploy backend-develop  --gitlab-project-id 123 --gitlab-project-path website-api                     --project-name website --reference develop
gitlab-deploy backend-deploy   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test
gitlab-deploy backend-control  --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --command up
gitlab-deploy simple-deploy    --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test
gitlab-deploy simple-control   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test -- sudo /usr/local/bin/apply-nginx.sh dev.env

Usage: gitlab-deploy <COMMAND>

Commands:
  frontend-develop  Fetch the project via GitLab API and then build it and use the public static files on a development host
  frontend-deploy   Fetch the project via GitLab API and then build it and deploy the archive of public static files on multiple hosts according to the phase
  frontend-control  Control the project on multiple hosts according to the phase
  backend-develop   Fetch the project via Git and checkout to a specific branch and then start up the service on a development host
  backend-deploy    Fetch the project via GitLab API and then build it and deploy the docker image on multiple hosts according to the phase
  backend-control   Control the project on multiple hosts according to the phase
  simple-deploy     Fetch the project via GitLab API and deploy the project files on multiple hosts according to the phase
  simple-control    Control the project on multiple hosts according to the phase
  help              Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## License

[MIT](LICENSE)
