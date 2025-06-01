#include <stddef.h>
#include <stdint.h>
#include <assert.h>
#include <Adafruit_NeoPixel.h>

/* 
 * ARDUINO AGENT FOR REACTIVE LED MATRIX / CURTAIN
 * ===============================================
 * 
 * - Set up wired to a raspberry pi. The serial ports are linked together and
 *   the raspberry pi feeds color data to the arduino through them.
 * - Arduino controls WS2812 light strip via pin 6
 */

#define LED_PIN    6
#define LED_WIDTH  20
#define LED_HEIGHT 26
#define LED_COUNT  520

#define RED_PIN    11
#define GREEN_PIN  10
#define BLUE_PIN   9

Adafruit_NeoPixel strip(LED_COUNT, LED_PIN, NEO_GRB + NEO_KHZ800);

void setup() {
  Serial.begin(115200, SERIAL_8N1);
  pinMode(LED_BUILTIN, OUTPUT);
  digitalWrite(LED_BUILTIN, HIGH);

  pinMode(RED_PIN, OUTPUT);
  pinMode(GREEN_PIN, OUTPUT);
  pinMode(BLUE_PIN, OUTPUT);

  strip.begin();
  strip.clear();
  strip.show();
  strip.setBrightness(50);
}

// https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing
/** COBS decode data from buffer
	@param buffer Pointer to encoded input bytes
	@param length Number of bytes to decode
	@param data Pointer to decoded output data
	@return Number of bytes successfully decoded
	@note Stops decoding if delimiter byte is found
*/
size_t cobsDecode(const uint8_t *buffer, size_t length, void *data)
{
	assert(buffer && data);

	const uint8_t *byte = buffer; // Encoded input byte pointer
	uint8_t *decode = (uint8_t *)data; // Decoded output byte pointer

	for (uint8_t code = 0xff, block = 0; byte < buffer + length; --block)
	{
		if (block) // Decode block byte
			*decode++ = *byte++;
		else
		{
			block = *byte++;             // Fetch the next block length
			if (block && (code != 0xff)) // Encoded zero, write it unless it's delimiter.
				*decode++ = 0;
			code = block;
			if (!code) // Delimiter code found
				break;
		}
	}

	return (size_t)(decode - (uint8_t *)data);
}

uint8_t encoded[256];
uint8_t decoded[256];

bool led = 1;

void loop() {
  if (Serial.available()) {
    size_t enc_len = Serial.readBytesUntil(0, &encoded[0], 256);
    encoded[enc_len] = 0;
    size_t dec_len = cobsDecode(&encoded[0], enc_len, &decoded[0]);

    // expected format: byte 0: the column of the LED matrix, bytes 1-78, the LED color data
    if (dec_len == LED_HEIGHT * 3 + 1) {
      uint8_t col = decoded[0];
      if (col >= LED_WIDTH) {
        return; // invalid column
      }

      for (int i = 0; i < LED_HEIGHT; i++) {
        strip.setPixelColor(
          col * LED_HEIGHT + i,
          decoded[1 + i * 3],
          decoded[2 + i * 3],
          decoded[3 + i * 3]
        );
      }
      // will refresh the led strip when on the last column
      if (col == LED_WIDTH - 1) {
        strip.show();
        strip.clear();
        led = !led;
        digitalWrite(LED_BUILTIN, led);
      }
    } 
    // debug: is COBS working?
    else if (dec_len = 8 && decoded[0] == 111) {
      digitalWrite(LED_BUILTIN, HIGH);
      // echo it back
      Serial.write(decoded, dec_len);
    }
    // debug: test RGB LED
    else if (dec_len = 3) {
      analogWrite(RED_PIN, decoded[0]);
      analogWrite(GREEN_PIN, decoded[1]);
      analogWrite(BLUE_PIN, decoded[2]);
    }
  }
}
