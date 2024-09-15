/**
 * @type {import('semantic-release').GlobalConfig}
 */
module.exports = {
  branches: ['main'],
  plugins: [
    '@semantic-release/commit-analyzer',
    '@semantic-release/release-notes-generator',
    [
      '@semantic-release/exec',
      {
        prepareCmd: '.github/pipeline/prepareCmd.sh ${nextRelease.version}',
        publishCmd: '.github/pipeline/dockerBuild.sh ${nextRelease.version}',
      },
    ],
    [
      '@semantic-release/github',
      {
        assets: [{ path: 'target/release/propeller' }],
      },
    ],
  ],
};
