import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.1/index.js';

// Custom metrics
export const errorRate = new Rate('errors');
export const apiResponseTime = new Trend('api_response_time');
export const apiRequests = new Counter('api_requests');
export const healthCheckFails = new Counter('health_check_fails');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:7878';
const API_KEY = __ENV.API_KEY || 'test-api-key';

// Test scenarios
export const options = {
  scenarios: {
    // Baseline test - 10 users for 1 minute
    baseline: {
      executor: 'constant-vus',
      vus: 10,
      duration: '1m',
      tags: { test_type: 'baseline' },
    },
    
    // Load test - ramp up to 100 users over 5 minutes
    load_test: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '1m', target: 25 },   // Warm up
        { duration: '2m', target: 100 },  // Ramp up to 100 users
        { duration: '2m', target: 100 },  // Stay at 100 users
        { duration: '1m', target: 0 },    // Ramp down
      ],
      tags: { test_type: 'load' },
    },
    
    // Spike test - sudden load increase
    spike_test: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 50 },  // Normal load
        { duration: '30s', target: 200 }, // Spike to 200 users
        { duration: '30s', target: 50 },  // Back to normal
        { duration: '30s', target: 0 },   // Ramp down
      ],
      tags: { test_type: 'spike' },
    },
    
    // Stress test - find breaking point
    stress_test: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '1m', target: 100 },  // Normal load
        { duration: '2m', target: 300 },  // High load
        { duration: '2m', target: 500 },  // Stress load
        { duration: '1m', target: 0 },    // Ramp down
      ],
      tags: { test_type: 'stress' },
    },
    
    // Soak test - sustained load for 30 minutes
    soak_test: {
      executor: 'constant-vus',
      vus: 50,
      duration: '30m',
      tags: { test_type: 'soak' },
    },
  },
  
  // Performance thresholds
  thresholds: {
    // Overall response time requirements
    http_req_duration: [
      'p(50)<50',     // 50% of requests under 50ms
      'p(95)<100',    // 95% of requests under 100ms
      'p(99)<200',    // 99% of requests under 200ms
    ],
    
    // API specific thresholds
    'api_response_time': ['p(95)<100'],
    
    // Error rate thresholds
    'errors': ['rate<0.01'],          // Error rate under 1%
    'http_req_failed': ['rate<0.01'], // HTTP failure rate under 1%
    
    // Throughput requirements
    'http_reqs': ['rate>1000'],       // At least 1000 req/s
    
    // Specific scenario thresholds
    'http_req_duration{test_type:baseline}': ['p(95)<50'],
    'http_req_duration{test_type:load}': ['p(95)<100'],
    'http_req_duration{test_type:spike}': ['p(95)<200'],
  },
  
  // Additional options
  userAgent: 'k6-load-test/1.0',
  insecureSkipTLSVerify: true,
  noConnectionReuse: false,
  noVUConnectionReuse: false,
};

// Test data
const testMovies = [
  { title: 'The Matrix', year: 1999, tmdbId: 603 },
  { title: 'Inception', year: 2010, tmdbId: 27205 },
  { title: 'Interstellar', year: 2014, tmdbId: 157336 },
  { title: 'The Dark Knight', year: 2008, tmdbId: 155 },
  { title: 'Pulp Fiction', year: 1994, tmdbId: 680 },
];

export function setup() {
  console.log('Starting performance test suite...');
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`API Key: ${API_KEY ? 'Set' : 'Not set'}`);
  
  // Verify the application is running
  const healthCheck = http.get(`${BASE_URL}/health`);
  if (healthCheck.status !== 200) {
    throw new Error(`Application not responding. Health check failed with status: ${healthCheck.status}`);
  }
  
  console.log('Application health check passed');
  return { baseUrl: BASE_URL, apiKey: API_KEY };
}

export default function (data) {
  const testType = __ITER % 5; // Rotate through different test types
  
  group('Health Check', () => {
    const response = http.get(`${data.baseUrl}/health`);
    
    const success = check(response, {
      'health check status is 200': (r) => r.status === 200,
      'health check response time < 50ms': (r) => r.timings.duration < 50,
      'health check has correct content': (r) => r.json('status') === 'healthy',
    });
    
    if (!success) {
      healthCheckFails.add(1);
    }
    
    errorRate.add(!success);
  });
  
  group('API Endpoints', () => {
    const headers = {
      'X-Api-Key': data.apiKey,
      'Content-Type': 'application/json',
    };
    
    // Test different API endpoints based on test type
    switch (testType) {
      case 0:
        testMoviesEndpoint(data.baseUrl, headers);
        break;
      case 1:
        testSearchEndpoint(data.baseUrl, headers);
        break;
      case 2:
        testCommandsEndpoint(data.baseUrl, headers);
        break;
      case 3:
        testCalendarEndpoint(data.baseUrl, headers);
        break;
      case 4:
        testDownloadsEndpoint(data.baseUrl, headers);
        break;
    }
  });
  
  group('Database Heavy Operations', () => {
    const headers = { 'X-Api-Key': data.apiKey };
    
    // Simulate complex database queries
    const batchRequests = [
      ['GET', `${data.baseUrl}/api/v3/movie`],
      ['GET', `${data.baseUrl}/api/v3/movie?limit=50&offset=0`],
      ['GET', `${data.baseUrl}/api/v3/calendar`],
    ];
    
    const responses = http.batch(batchRequests.map(([method, url]) => [method, url, null, { headers }]));
    
    responses.forEach((response, index) => {
      const endpoint = batchRequests[index][1].split('/').pop();
      
      check(response, {
        [`${endpoint} status is 200`]: (r) => r.status === 200,
        [`${endpoint} response time < 200ms`]: (r) => r.timings.duration < 200,
      });
      
      apiResponseTime.add(response.timings.duration);
      apiRequests.add(1);
      errorRate.add(response.status !== 200);
    });
  });
  
  // Random sleep between 1-3 seconds to simulate user behavior
  sleep(Math.random() * 2 + 1);
}

function testMoviesEndpoint(baseUrl, headers) {
  group('Movies API', () => {
    // GET all movies
    let response = http.get(`${baseUrl}/api/v3/movie`, { headers });
    
    check(response, {
      'movies list status is 200': (r) => r.status === 200,
      'movies list response time < 100ms': (r) => r.timings.duration < 100,
    });
    
    apiResponseTime.add(response.timings.duration);
    apiRequests.add(1);
    errorRate.add(response.status !== 200);
    
    // POST new movie (simulation)
    const testMovie = testMovies[Math.floor(Math.random() * testMovies.length)];
    response = http.post(`${baseUrl}/api/v3/movie`, JSON.stringify({
      title: testMovie.title,
      year: testMovie.year,
      tmdbId: testMovie.tmdbId,
      qualityProfileId: 1,
      rootFolderPath: '/movies',
      monitored: true,
    }), { headers });
    
    check(response, {
      'movie creation attempt processed': (r) => r.status === 200 || r.status === 201 || r.status === 400,
      'movie creation response time < 200ms': (r) => r.timings.duration < 200,
    });
    
    apiResponseTime.add(response.timings.duration);
    apiRequests.add(1);
    errorRate.add(![200, 201, 400].includes(response.status));
  });
}

function testSearchEndpoint(baseUrl, headers) {
  group('Search API', () => {
    const searchTerms = ['matrix', 'inception', 'batman', 'star wars', 'marvel'];
    const term = searchTerms[Math.floor(Math.random() * searchTerms.length)];
    
    const response = http.get(`${baseUrl}/api/v3/movie/search?term=${term}`, { headers });
    
    check(response, {
      'search status is 200': (r) => r.status === 200,
      'search response time < 300ms': (r) => r.timings.duration < 300,
    });
    
    apiResponseTime.add(response.timings.duration);
    apiRequests.add(1);
    errorRate.add(response.status !== 200);
  });
}

function testCommandsEndpoint(baseUrl, headers) {
  group('Commands API', () => {
    // GET commands status
    let response = http.get(`${baseUrl}/api/v3/command`, { headers });
    
    check(response, {
      'commands status is 200': (r) => r.status === 200,
      'commands response time < 100ms': (r) => r.timings.duration < 100,
    });
    
    apiResponseTime.add(response.timings.duration);
    apiRequests.add(1);
    errorRate.add(response.status !== 200);
  });
}

function testCalendarEndpoint(baseUrl, headers) {
  group('Calendar API', () => {
    const startDate = new Date().toISOString().split('T')[0];
    const endDate = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0];
    
    const response = http.get(`${baseUrl}/api/v3/calendar?start=${startDate}&end=${endDate}`, { headers });
    
    check(response, {
      'calendar status is 200': (r) => r.status === 200,
      'calendar response time < 150ms': (r) => r.timings.duration < 150,
    });
    
    apiResponseTime.add(response.timings.duration);
    apiRequests.add(1);
    errorRate.add(response.status !== 200);
  });
}

function testDownloadsEndpoint(baseUrl, headers) {
  group('Downloads API', () => {
    const response = http.get(`${baseUrl}/api/v3/downloads`, { headers });
    
    check(response, {
      'downloads status is 200': (r) => r.status === 200,
      'downloads response time < 100ms': (r) => r.timings.duration < 100,
    });
    
    apiResponseTime.add(response.timings.duration);
    apiRequests.add(1);
    errorRate.add(response.status !== 200);
  });
}

export function teardown(data) {
  console.log('Performance test completed');
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'scripts/perf/k6-results.json': JSON.stringify(data),
    'scripts/perf/k6-results.html': generateHTMLReport(data),
  };
}

function generateHTMLReport(data) {
  const timestamp = new Date().toISOString();
  
  return `
<!DOCTYPE html>
<html>
<head>
    <title>K6 Performance Test Results</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .metric { margin: 10px 0; padding: 10px; border-left: 4px solid #007acc; }
        .passed { border-left-color: #28a745; }
        .failed { border-left-color: #dc3545; }
        .summary { background: #f8f9fa; padding: 15px; border-radius: 5px; }
        table { width: 100%; border-collapse: collapse; margin: 15px 0; }
        th, td { padding: 8px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background-color: #f2f2f2; }
    </style>
</head>
<body>
    <h1>Radarr MVP Performance Test Results</h1>
    <p><strong>Test Date:</strong> ${timestamp}</p>
    
    <div class="summary">
        <h2>Executive Summary</h2>
        <p><strong>Total Requests:</strong> ${data.metrics.http_reqs.values.count}</p>
        <p><strong>Request Rate:</strong> ${data.metrics.http_reqs.values.rate.toFixed(2)} req/s</p>
        <p><strong>Average Response Time:</strong> ${data.metrics.http_req_duration.values.avg.toFixed(2)} ms</p>
        <p><strong>Error Rate:</strong> ${(data.metrics.http_req_failed.values.rate * 100).toFixed(2)}%</p>
    </div>
    
    <h2>Response Time Metrics</h2>
    <table>
        <tr><th>Metric</th><th>Value</th><th>Threshold</th><th>Status</th></tr>
        <tr><td>P50</td><td>${data.metrics.http_req_duration.values.p50.toFixed(2)} ms</td><td>&lt; 50ms</td><td>${data.metrics.http_req_duration.values.p50 < 50 ? 'PASS' : 'FAIL'}</td></tr>
        <tr><td>P95</td><td>${data.metrics.http_req_duration.values.p95.toFixed(2)} ms</td><td>&lt; 100ms</td><td>${data.metrics.http_req_duration.values.p95 < 100 ? 'PASS' : 'FAIL'}</td></tr>
        <tr><td>P99</td><td>${data.metrics.http_req_duration.values.p99.toFixed(2)} ms</td><td>&lt; 200ms</td><td>${data.metrics.http_req_duration.values.p99 < 200 ? 'PASS' : 'FAIL'}</td></tr>
    </table>
    
    <h2>All Metrics</h2>
    <table>
        <tr><th>Metric</th><th>Count</th><th>Rate</th><th>Avg</th><th>Min</th><th>Max</th><th>P95</th></tr>
        ${Object.entries(data.metrics).map(([name, metric]) => `
            <tr>
                <td>${name}</td>
                <td>${metric.values.count || '-'}</td>
                <td>${metric.values.rate ? metric.values.rate.toFixed(2) : '-'}</td>
                <td>${metric.values.avg ? metric.values.avg.toFixed(2) : '-'}</td>
                <td>${metric.values.min ? metric.values.min.toFixed(2) : '-'}</td>
                <td>${metric.values.max ? metric.values.max.toFixed(2) : '-'}</td>
                <td>${metric.values.p95 ? metric.values.p95.toFixed(2) : '-'}</td>
            </tr>
        `).join('')}
    </table>
</body>
</html>
  `;
}