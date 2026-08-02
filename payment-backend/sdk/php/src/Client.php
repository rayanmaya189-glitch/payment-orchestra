<?php

declare(strict_types=1);

namespace PaymentOrchestra;

use GuzzleHttp\Client as HttpClient;
use GuzzleHttp\Exception\RequestException;

/**
 * Payment Orchestration Platform Client
 *
 * Example:
 * $client = new PaymentOrchestra('pk_test_key', 'sk_test_key');
 * $payment = $client->paymentIntents->create(['amount' => 10000, 'currency' => 'USD']);
 */
class Client
{
    private string $apiKey;
    private string $secretKey;
    private string $baseUrl;
    private HttpClient $httpClient;

    public PaymentIntents $paymentIntents;
    public GatewayProfiles $gatewayProfiles;
    public RoutingPolicies $routingPolicies;
    public Webhooks $webhooks;

    public function __construct(
        string $apiKey,
        string $secretKey,
        string $environment = 'sandbox',
        ?string $baseUrl = null,
        ?HttpClient $httpClient = null
    ) {
        $this->apiKey = $apiKey;
        $this->secretKey = $secretKey;

        if ($baseUrl) {
            $this->baseUrl = $baseUrl;
        } elseif ($environment === 'production') {
            $this->baseUrl = 'https://api.paymentorchestra.com';
        } else {
            $this->baseUrl = 'https://sandbox.api.paymentorchestra.com';
        }

        $this->httpClient = $httpClient ?: new HttpClient([
            'base_uri' => $this->baseUrl,
            'timeout' => 30,
            'headers' => [
                'Content-Type' => 'application/json',
                'Authorization' => 'Bearer ' . $apiKey,
                'X-Secret-Key' => $secretKey,
                'X-SDK-Version' => '1.0.0',
                'X-SDK-Name' => 'php',
            ],
        ]);

        $this->paymentIntents = new PaymentIntents($this);
        $this->gatewayProfiles = new GatewayProfiles($this);
        $this->routingPolicies = new RoutingPolicies($this);
        $this->webhooks = new Webhooks($this);
    }

    /**
     * Make an API request
     */
    public function request(string $method, string $path, array $data = []): array
    {
        try {
            $options = [];
            if (!empty($data)) {
                $options['json'] = $data;
            }

            $response = $this->httpClient->request($method, $path, $options);
            $body = (string) $response->getBody();

            return json_decode($body, true) ?? [];
        } catch (RequestException $e) {
            $response = $e->getResponse();
            $statusCode = $response ? $response->getStatusCode() : 0;
            $body = $response ? (string) $response->getBody() : $e->getMessage();

            throw new APIException($statusCode, $body);
        }
    }

    /**
     * Get the base URL
     */
    public function getBaseUrl(): string
    {
        return $this->baseUrl;
    }
}
