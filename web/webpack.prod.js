const path = require('path');
const { merge } = require('webpack-merge');
const common = require('./webpack.common.js');

module.exports = merge(common, {
  mode: 'production',
  output: {
    filename: '[id].bundle.js',
    path: path.resolve(__dirname, './dist/prod/static'),
    clean: true,
  },
  performance: {
    hints: false,
  },
  optimization: {
    innerGraph: true,
    splitChunks: {
      chunks: 'all',
    },
  },
});
