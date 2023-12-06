const EXTERNAL_PIP_INDEX_URL = process.env.EXTERNAL_PIP_INDEX_URL;

const branches = [
    'master',
    {
        name: 'alpha',
        prerelease: true
    },
    {
        name: 'fix-build',
        prerelease: "rc"
    }
];

const commitAnalyzer = '@semantic-release/commit-analyzer';

const git = '@semantic-release/git';

const exec = [
    "@semantic-release/exec",
    {
        "prepareCmd": "echo \"RELEASE_VERSION=${nextRelease.version}\" >> $GITHUB_ENV"
    }
];

module.exports = {
    branches,
    plugins: [commitAnalyzer, git, exec]
};