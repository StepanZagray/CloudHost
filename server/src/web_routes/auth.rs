use axum::response::Html;

// Login page
pub async fn login_page() -> Html<String> {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>CloudTUI Login</title>
    <style>
        body { 
            font-family: Arial, sans-serif; 
            margin: 0; 
            padding: 0; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .login-container {
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0,0,0,0.1);
            width: 100%;
            max-width: 400px;
        }
        .login-header {
            text-align: center;
            margin-bottom: 30px;
        }
        .login-header h1 {
            color: #333;
            margin: 0;
            font-size: 28px;
        }
        .login-header p {
            color: #666;
            margin: 10px 0 0 0;
        }
        .form-group {
            margin-bottom: 20px;
        }
        .form-group label {
            display: block;
            margin-bottom: 5px;
            color: #333;
            font-weight: bold;
        }
        .form-group input {
            width: 100%;
            padding: 12px;
            border: 2px solid #ddd;
            border-radius: 5px;
            font-size: 16px;
            box-sizing: border-box;
        }
        .form-group input:focus {
            outline: none;
            border-color: #667eea;
        }
        .login-button {
            width: 100%;
            padding: 12px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 5px;
            font-size: 16px;
            cursor: pointer;
            transition: transform 0.2s;
        }
        .login-button:hover {
            transform: translateY(-2px);
        }
        .error-message {
            color: #e74c3c;
            text-align: center;
            margin-top: 15px;
            padding: 10px;
            background: #fdf2f2;
            border-radius: 5px;
            display: none;
        }
        .success-message {
            color: #27ae60;
            text-align: center;
            margin-top: 15px;
            padding: 10px;
            background: #f0f9f0;
            border-radius: 5px;
            display: none;
        }
    </style>
</head>
<body>
    <div class="login-container">
        <div class="login-header">
            <h1>🌩️ CloudTUI</h1>
            <p>Enter your password to access your cloud storage</p>
        </div>
        <form id="loginForm">
            <div class="form-group">
                <label for="password">Password:</label>
                <input type="password" id="password" name="password" required>
            </div>
            <button type="submit" class="login-button">Login</button>
        </form>
        <div id="errorMessage" class="error-message"></div>
        <div id="successMessage" class="success-message"></div>
    </div>

    <script>
        document.getElementById('loginForm').addEventListener('submit', async function(e) {
            e.preventDefault();
            
            const password = document.getElementById('password').value;
            const errorDiv = document.getElementById('errorMessage');
            const successDiv = document.getElementById('successMessage');
            
            // Hide previous messages
            errorDiv.style.display = 'none';
            successDiv.style.display = 'none';
            
            try {
                const response = await fetch('/api/login', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ password: password })
                });
                
                if (response.ok) {
                    const data = await response.json();
                    // Store token in cookie
                    document.cookie = `auth_token=${data.token}; path=/; max-age=86400`; // 24 hours
                    successDiv.textContent = 'Login successful! Redirecting...';
                    successDiv.style.display = 'block';
                    
                    // Redirect to home page
                    setTimeout(() => {
                        window.location.href = '/';
                    }, 1000);
                } else {
                    const error = await response.message.text();
                    errorDiv.textContent = error;
                    errorDiv.style.display = 'block';
                }
            } catch (error) {
                errorDiv.textContent = 'Login failed. Please try again.';
                errorDiv.style.display = 'block';
            }
        });
    </script>
</body>
</html>
    "#;
    Html(html.to_string())
}
