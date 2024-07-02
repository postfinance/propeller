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
        prepareCmd: 'cargo bump ${nextRelease.version} && cargo build --release',
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
