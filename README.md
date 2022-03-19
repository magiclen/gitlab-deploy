Gitlab Deploy
====================

[![CI](https://github.com/magiclen/gitlab-deploy/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/gitlab-deploy/actions/workflows/ci.yml)

GitLab Deploy is used for deploying software projects to multiple hosts during different phases. This program should be run on Linux.

## Setup

TBD

## Help

```
EXAMPLES:
gitlab-deploy frontend-develop   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --build-target develop
gitlab-deploy frontend-deploy    --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --build-target test
gitlab-deploy frontend-control   --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test
gitlab-deploy backend-develop    --gitlab-project-id 123 --gitlab-project-path website-api                     --project-name website --reference develop
gitlab-deploy backend-deploy     --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test
gitlab-deploy backend-control    --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test --command up
gitlab-deploy simple-deploy      --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test
gitlab-deploy simple-control     --gitlab-project-id 123 --commit-sha 0b14cd4fdec3bdffffdaf1de6fe13aaa01c4827f --project-name website --reference-name pre-release --phase test sudo /usr/local/bin/apply-nginx.sh dev.env

USAGE:
    gitlab-deploy [SUBCOMMAND]

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    frontend-develop    Fetch the project via GitLab API and then build it and use the public static files on a development host
    frontend-deploy     Fetch the project via GitLab API and then build it and deploy the archive of public static files on multiple hosts according to the phase
    frontend-control    Control the project on multiple hosts according to the phase
    backend-develop     Fetch the project via Git and checkout to a specific branch and then start up the service on a development host
    backend-deploy      Fetch the project via GitLab API and then build it and deploy the docker image on multiple hosts according to the phase
    backend-control     Control the project on multiple hosts according to the phase
    simple-deploy       Fetch the project via GitLab API and deploy the project files on multiple hosts according to the phase
    simple-control      Control the project on multiple hosts according to the phase
    help                Print this message or the help of the given subcommand(s)
```

## License

[MIT](LICENSE)