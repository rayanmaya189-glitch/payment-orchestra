<?php

declare(strict_types=1);

namespace PaymentOrchestra;

/**
 * API Exception
 */
class APIException extends \Exception
{
    private int $statusCode;
    private string $responseBody;

    public function __construct(int $statusCode, string $responseBody)
    {
        $this->statusCode = $statusCode;
        $this->responseBody = $responseBody;

        $message = "API Error [$statusCode]: $responseBody";
        parent::__construct($message, $statusCode);
    }

    public function getStatusCode(): int
    {
        return $this->statusCode;
    }

    public function getResponseBody(): string
    {
        return $this->responseBody;
    }
}
