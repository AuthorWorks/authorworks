# Legacy Source Code

This directory contains the complete source code from the original monolithic application. 
It is provided as a reference for implementing the microservice according to the specifications.

## Usage Guidelines

1. **Do not** directly use this code in your microservice implementation
2. **Do** use it as a reference for understanding the existing business logic
3. **Do** refactor and adapt the code to fit the microservice architecture
4. **Do** follow the specifications in the `specs/` directory

## Migration Strategy

Follow the "Strangler Fig Pattern" as described in Martin Fowler's approach:
1. Identify the boundaries of the functionality you need to implement
2. Create a new implementation using modern practices
3. Gradually replace functionality while ensuring compatibility
4. Remove dependencies on the legacy code

For more information on refactoring monoliths to microservices, see:
[Refactoring a Monolith into Microservices](https://www.nginx.com/blog/refactoring-a-monolith-into-microservices/)
