from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import parse_qs
import subprocess, sys
from subprocess import Popen, PIPE
import uuid
import json
from inspect import getmembers
from pprint import pprint
import sys
import threading
import queue

front_base = b"""<!DOCTYPE html>
<html>
  <head>
    <style>
      h1, h2, h3, h4, h5, h6 {
        font-family: Georgia, Times, serif;
      }
      p, div {
        font-family: Helvetica, Arial, sans-serif;
      }
      body {
        background-color:#24292e;
      }
      #content {
        margin: auto;
        margin-top:20px;
        width: 50%;
        border: 1px solid #bedfff;
        border-radius: 15px;
        padding: 10px;
        background-color:#fff;
      }
      ul {
        list-style-type: none;
      }
      form {
            max-width: 400px;
            margin: 0 auto;
        }

        label {
            display: inline-block;
            width: 150px; /* Adjust the width as needed */
            margin-bottom: 5px;
        }

        select {
            width: 100%;
            padding: 5px;
            margin-bottom: 10px;
            box-sizing: border-box;
        }

        input[type="submit"] {
            width: 100%;
            padding: 10px;
            box-sizing: border-box;
        }

        #overlay {
            display: none;
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.7); /* Semi-transparent black */
            justify-content: center;
            align-items: center;
            z-index: 999; /* Make sure the overlay is on top */
        }

        #loading-container {
            text-align: center;
            color: white;
        }

        #loading-message {
            background-color: #333; /* Solid background color */
            padding: 20px; /* Padding to make it smaller */
            border-radius: 10px; /* Rounded corners */
            margin-bottom: 10px;
            text-align: left;
        }

        #exit-button {
            background-color: #333;
            color: white;
            padding: 10px;
            border: none;
            border-radius: 5px;
            cursor: pointer;
            display: none;
        }
    </style>
    <title>Loan Calculator</title>
    <script>
    	function showOverlay() {
	        document.getElementById('overlay').style.display = 'flex';
	    }

	    function hideOverlay() {
	        document.getElementById('overlay').style.display = 'none';
	        document.getElementById('loading-message').innerHTML = 'Processing your query. This can take 1-3 minutes.<br/>'
	        document.getElementById('exit-button').style.display = 'none';
	    }

    	handlers = {}
    	async function handle_form() {
    		var gender = document.getElementById('gender').value;
	        var contractType = document.getElementById('contract_type').value;
	        var emergencyState = document.getElementById('emergency_state').value;
	        var educationLevel = document.getElementById('education_level').value;
	        var incomeType = document.getElementById('income_type').value;
	        var houseType = document.getElementById('house_type').value;
	        var ownCar = document.getElementById('own_car').value;
	        var familyStatus = document.getElementById('family_status').value;
    		getProcessCode = async () => {
			    const location = window.location.hostname;
			    const settings = {
			        method: 'POST',
			        headers: {
			            Accept: 'application/json',
			            'Content-Type': 'application/json',
			        },
			        body: JSON.stringify({
			        	"gender": gender,
			        	"contract_type": contractType,
			        	"emergency_state": emergencyState,
			        	"education_level": educationLevel,
			        	"income_type": incomeType,
			        	"house_type": houseType,
			        	"own_car": ownCar,
			        	"family_status": familyStatus
			        })
			    };
			    try {
			        const fetchResponse = await fetch(`http://${location}:8000/start`, settings);
			        const data = await fetchResponse.json();
			        return data;
			    } catch (e) {
			        return e;
			    }    
			}

			//get_status((await getProcessCode()).id)
			showOverlay();
			let process_code = (await getProcessCode()).id
			handlers[process_code] = setInterval(() => {
				get_status(process_code)
			}, 3000)
    	}

    	async function get_status(process_code) {
    		queryProcessStatus = async () => {
			    const location = window.location.hostname;
			    const settings = {
			        method: 'POST',
			        headers: {
			            Accept: 'application/json',
			            'Content-Type': 'application/json',
			        },
			        body: JSON.stringify({
			        	id: process_code
			        })
			    };
			    try {
			        const fetchResponse = await fetch(`http://${location}:8000/status`, settings);
			        const data = await fetchResponse.json();
			        return data;
			    } catch (e) {
			        return e;
			    }    
			}


			let process = await queryProcessStatus();
			if (process.completed) {
				clearInterval(handlers[process_code])
				document.getElementById('exit-button').style.display = 'inline-block';
				get_status(process_code)
			}

			if (process.msg != undefined) {
				if (process.msg.startsWith("Gender")) {return;} // skip the prompts for input
				var loading = document.getElementById("loading-message");
				loading.innerHTML += process.msg + "<br/>";
			}
    	}
    </script>
  </head>
  <body>
  	<div id="overlay">
  		<div id="loading-container">
		    <div id="loading-message">Processing your query. This can take 1-3 minutes.<br/></div>
		    <button id="exit-button" onclick="hideOverlay()">Submit New Query</button>
	    </div>
	</div>

  	<div id ="content">
  		<h1 style = "text-align:center;">Loan Risk Calculator</h1>
  		"""

back_base = b"""
	</div>
  </body>
</html>"""


process_table = {}

def non_blocking_readline(fd, output_queue):
    for line in iter(fd.readline, b''):
        output_queue.put(line)

def do_status(handler):
	length = int(handler.headers.get('Content-length'))
	field_data = handler.rfile.read(length)
	fields = json.loads(str(field_data,"UTF-8"))
	resp = {}
	resp['id'] = fields['id']
	process_handle = process_table[fields['id']]
	process = process_handle['process']
	output = process_handle['output']

	resp['completed'] = (not (process.poll() is None)) and (not output.empty())

	if not hasattr(process, 'reader_thread') or not process.reader_thread.is_alive():
		process.reader_thread = threading.Thread(target=non_blocking_readline, args=(process.stdout, output))
		process.reader_thread.start()

	try:
		line = output.get_nowait().decode('utf-8')
		resp['msg'] = str(line).rstrip('\n')
	except queue.Empty:
		line = ""

	handler.wfile.write(str.encode(json.dumps(resp)))

def do_process(handler):
	length = int(handler.headers.get('Content-length'))
	field_data = handler.rfile.read(length)
	fields = json.loads(str(field_data,"UTF-8"))
	process = subprocess.Popen(['target/release/mining', 'application_data.csv'], stdout=PIPE, stderr=PIPE, stdin=PIPE)
	id = uuid.uuid4().hex
	process_table[id] = {
			'process': process,
			'output': queue.Queue()
	}

	print(fields)
	process.stdin.write(str.encode(fields['gender'] + '\n'))
	process.stdin.flush()
	process.stdin.write(str.encode(fields['contract_type'] + '\n'))
	process.stdin.flush()
	process.stdin.write(str.encode(fields['emergency_state'] + '\n'))
	process.stdin.flush()
	process.stdin.write(str.encode(fields['education_level'] + '\n'))
	process.stdin.flush()
	process.stdin.write(str.encode(fields['income_type'] + '\n'))
	process.stdin.flush()
	process.stdin.write(str.encode(fields['house_type'] + '\n'))
	process.stdin.flush()
	process.stdin.write(str.encode(fields['own_car'] + '\n'))
	process.stdin.flush()
	process.stdin.write(str.encode(fields['family_status'] + '\n'))
	process.stdin.flush()

	handler.wfile.write(str.encode('{"id": "'+id+'"}'))

class ReqHandler(BaseHTTPRequestHandler):
	def do_GET(self):
		self.send_response(200)
		self.send_header('Content-type', 'text/html')
		self.end_headers()
		self.wfile.write(front_base)
		self.wfile.write(b"""
			<form onsubmit="handle_form()" action="#" method="get">
			    <label for="gender">Gender:</label>
			    <select id="gender" name="gender">
			        <option value="M">M</option>
			        <option value="F">F</option>
			        <option value="XNA">XNA</option>
			    </select>
			    <br>

			    <label for="contract_type">Contract Type:</label>
			    <select id="contract_type" name="contract_type">
			        <option value="Cash loans">Cash loans</option>
			        <option value="Revolving loans">Revolving loans</option>
			    </select>
			    <br>

			    <label for="emergency_state">Emergency State:</label>
			    <select id="emergency_state" name="emergency_state">
			        <option value="Yes">Yes</option>
			        <option value="No">No</option>
			    </select>
			    <br>

			    <label for="education_level">Education Level:</label>
			    <select id="education_level" name="education_level">
			        <option value="Lower secondary">Lower secondary</option>
			        <option value="Secondary / secondary special">Secondary / secondary special</option>
			        <option value="Incomplete higher">Incomplete higher</option>
			        <option value="Higher education">Higher education</option>
			        <option value="Academic degree">Academic degree</option>
			    </select>
			    <br>

			    <label for="income_type">Income Type:</label>
			    <select id="income_type" name="income_type">
			        <option value="Unemployed">Unemployed</option>
			        <option value="Maternity leave">Maternity leave</option>
			        <option value="Pensioner">Pensioner</option>
			        <option value="Working">Working</option>
			        <option value="Student">Student</option>
			        <option value="State servant">State servant</option>
			        <option value="Businessman">Businessman</option>
			        <option value="Commercial associate">Commercial associate</option>
			    </select>
			    <br>

			    <label for="house_type">House Type:</label>
			    <select id="house_type" name="house_type">
			        <option value="N/A">N/A</option>
			        <option value="specific housing">specific housing</option>
			        <option value="terraced house">terraced house</option>
			        <option value="block of flats">block of flats</option>
			    </select>
			    <br>

			    <label for="own_car">Own Car?</label>
			    <select id="own_car" name="own_car">
			        <option value="Y">Y</option>
			        <option value="N">N</option>
			    </select>
			    <br>

			    <label for="family_status">Family Status:</label>
			    <select id="family_status" name="family_status">
			        <option value="Single">Single</option>
			        <option value="Married">Married</option>
			        <option value="Civilly married">Civilly married</option>
			        <option value="Separated">Separated</option>
			        <option value="Widow">Widow</option>
			        <option value="Unknown">Unknown</option>
			    </select>
			    <br>

			    <input type="submit" value="Submit" onclick="handle_form(); return false">
			</form>
		""")
		self.wfile.write(back_base)

	def do_POST(self):
		if self.path == '/start':
			self.send_response(200)
			self.send_header('Content-type', 'text/json')
			self.end_headers()
			do_process(self)
		elif self.path == '/status':
			self.send_response(200)
			self.send_header('Content-type', 'text/json')
			self.end_headers()
			do_status(self)
		else:
			self.send_response(404)
			self.send_header('Content-type', 'text/json')
			self.end_headers()
			self.wfile.write(b'{"err": "Not found"}')
		

server_address = ('', 8000)
httpd = HTTPServer(server_address, ReqHandler)
print(f'Starting server on port 8000.')
httpd.serve_forever()
