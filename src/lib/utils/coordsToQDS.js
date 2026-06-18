/**
 * 
 * @param {*} latitude string
 * @param {*} longitude string
 * @returns {string|null} QDS code or null if invalid
 */
export function coordsToQDS(latitude, longitude) {

  if ((latitude == null && longitude != null) || (latitude != null && longitude == null)) {
    throw new Error('Invalid latitude and longitude')
  }

  if (isNaN(Number(latitude)) || isNaN(Number(longitude))) {
    throw new Error('Invalid latitude and longitude')
  }

  if (latitude == null || longitude == null) {
    return null
  }

  latitude = Number(latitude)
  longitude = Number(longitude) // not really necessary

  const westernLimit = 8.6; // Angola
  const easternLimit = 63.7; // Rodrigues

  //QDS are only valid in southern Africa, but we'll be liberal and allow further afield for now
  if (latitude > 0.0 || longitude < westernLimit || longitude > easternLimit) {
    return null
  }

  latitude = Math.abs(latitude).toFixed(6).replace(',', '.').trim()
  longitude = Math.abs(longitude).toFixed(6).replace(',', '.').trim() // not really necessary

  let qds = [latitude, longitude].map(x => x.split('.')[0].toString().padStart(2, '0')).join('')
  let latDecimalPart = (Number(latitude) - Number(latitude.split('.')[0])) * 100
  let longDecimalPart = (Number(longitude) - Number(longitude.split('.')[0])) * 100
  const row = latDecimalPart < 50 ? 'AB' : 'CD'
  const first = longDecimalPart < 50 ? row[0] : row[1]

  if (latDecimalPart >= 50) {
    latDecimalPart -= 50
  }

  if (longDecimalPart >= 50) {
    longDecimalPart -= 50
  }

  const row2 = latDecimalPart < 25 ? 'AB' : 'CD'
  const second = longDecimalPart < 25 ? row2[0] : row2[1]

  qds += first + second
  return qds
}
